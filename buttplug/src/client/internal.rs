use crate::core::messages::{self, ButtplugMessageUnion};
use super::connector::{ButtplugClientConnector, ButtplugClientConnectorError};
use core::pin::Pin;
use futures::{select, FutureExt, SinkExt, StreamExt, Future, task::{Waker, Poll, Context}, future::Fuse};
use futures_channel::mpsc;
use std::sync::{Arc, Mutex};

#[derive(Default, Debug, Clone)]
pub struct ButtplugClientMessageState {
    reply_msg: Option<ButtplugMessageUnion>,
    waker: Option<Waker>,
}

impl ButtplugClientMessageState {
    pub fn set_reply_msg(&mut self, msg: &ButtplugMessageUnion) {
        self.reply_msg = Some(msg.clone());
        let waker = self.waker.take();
        if !waker.is_none() {
            waker.unwrap().wake();
        }
    }
}

pub type ButtplugClientMessageStateShared = Arc<Mutex<ButtplugClientMessageState>>;

#[derive(Default, Debug)]
pub struct ButtplugClientMessageFuture {
    // This needs to be an Arc<Mutex<T>> in order to make it mutable under
    // pinning when dealing with being a future. There is a chance we could do
    // this as a unchecked_mut borrow from pin, which would be way faster, but
    // that's dicey and hasn't been proven as needed for speed yet.
    waker_state: ButtplugClientMessageStateShared,
}

impl ButtplugClientMessageFuture {
    pub fn new(state: &ButtplugClientMessageStateShared) -> ButtplugClientMessageFuture {
        ButtplugClientMessageFuture {
            waker_state: state.clone(),
        }
    }

    pub fn get_state_ref(&self) -> &ButtplugClientMessageStateShared {
        &self.waker_state
    }

    // TODO Should we implement drop on this, so it'll yell if its dropping and
    // the waker didn't fire? otherwise it seems like we could have quiet
    // deadlocks.
}

impl Future for ButtplugClientMessageFuture {
    type Output = ButtplugMessageUnion;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut waker_state = self.waker_state.lock().unwrap();
        if waker_state.reply_msg.is_some() {
            let msg = waker_state.reply_msg.take().unwrap();
            Poll::Ready(msg)
        } else {
            println!("Got waker!");
            waker_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub struct ButtplugClientInternalLoop {
    connected_devices: Vec<u32>,
    connector: Option<Box<dyn ButtplugClientConnector>>,
    connector_receiver: Option<mpsc::UnboundedReceiver<ButtplugMessageUnion>>,
    client_sender: mpsc::UnboundedSender<ButtplugInternalClientMessage>,
    client_receiver: mpsc::UnboundedReceiver<ButtplugInternalClientMessage>,
    event_sender: Vec<mpsc::UnboundedSender<ButtplugMessageUnion>>,
}

unsafe impl Send for ButtplugClientInternalLoop {}

pub enum ButtplugInternalClientMessage {
    Connect(Box<dyn ButtplugClientConnector>, ButtplugClientMessageStateShared),
    Disconnect,
    Message((ButtplugMessageUnion, ButtplugClientMessageStateShared)),
    NewClient(mpsc::UnboundedSender<ButtplugMessageUnion>)
}

pub enum ButtplugInternalDeviceMessage {
    Message,
}

impl ButtplugClientInternalLoop {
    pub fn new(event_sender: mpsc::UnboundedSender<ButtplugMessageUnion>) -> Self {
        let (cs, cr) = mpsc::unbounded();
        ButtplugClientInternalLoop {
            connector: None,
            connected_devices: vec!(),
            connector_receiver: None,
            client_sender: cs,
            client_receiver: cr,
            event_sender: vec!(event_sender),
        }
    }

    pub fn get_client_sender(&self) -> mpsc::UnboundedSender<ButtplugInternalClientMessage> {
        self.client_sender.clone()
    }

    pub async fn wait_for_event(&mut self) -> Option<ButtplugClientConnectorError> {
        let mut event_future = Fuse::terminated();

        if let Some(ref mut recv) = self.connector_receiver {
            event_future = recv.next().fuse();
        }
        enum StreamReturn {
            ConnectorMessage(ButtplugMessageUnion),
            ClientMessage(ButtplugInternalClientMessage),
        }
        let mut client_future = self.client_receiver.next();
        let stream_ret = select! {
            a = event_future => StreamReturn::ConnectorMessage(a.unwrap()),
            b = client_future => StreamReturn::ClientMessage(b.unwrap()),
        };
        match stream_ret {
            StreamReturn::ConnectorMessage(_msg) => {
                for ref mut sender in self.event_sender.iter() {
                    println!("Sending message to clients!");
                    sender.send(_msg.clone()).await;
                }
                None
            },
            StreamReturn::ClientMessage(_msg) => {
                match _msg {
                    ButtplugInternalClientMessage::Connect(mut connector, mut state) => {
                        match connector.connect().await {
                            Some(_s) => {
                                None //return Result::Err(ButtplugClientError::ButtplugClientConnectorError(_s)),
                            },
                            None => {
                                println!("Connected!");
                                let mut waker_state = state.lock().unwrap();
                                waker_state.set_reply_msg(&ButtplugMessageUnion::Ok(messages::Ok::new(1)));
                                self.connector_receiver = Some(connector.get_event_receiver());
                                self.connector = Option::Some(connector);
                                None
                            }
                        }
                    },
                    ButtplugInternalClientMessage::Message(_msg_fut) => {
                        if let Some(ref mut connector) = self.connector {
                            connector.send(&_msg_fut.0, &_msg_fut.1).await;
                        }
                        None
                    },
                    ButtplugInternalClientMessage::NewClient(_sender) => {
                        self.event_sender.push(_sender);
                        None
                    },
                    _ => panic!("Message not handled!")
                }
            },
        }
    }
}

