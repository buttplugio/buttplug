// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::messages::{self, ButtplugMessageUnion};
use super::connector::{ButtplugClientConnector, ButtplugClientConnectorError};
use core::pin::Pin;
use futures::{StreamExt, Future, task::{Waker, Poll, Context}};
use async_std::{sync::{channel, Sender, Receiver}, future::{select}, task};
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
            info!("Got waker!");
            waker_state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub struct ButtplugClientInternalLoop {
    connected_devices: Vec<u32>,
    connector: Option<Box<dyn ButtplugClientConnector>>,
    connector_receiver: Option<Receiver<ButtplugMessageUnion>>,
    client_sender: Sender<ButtplugInternalClientMessage>,
    client_receiver: Receiver<ButtplugInternalClientMessage>,
    event_sender: Vec<Sender<ButtplugMessageUnion>>,
}

unsafe impl Send for ButtplugClientInternalLoop {}

pub enum ButtplugInternalClientMessage {
    Connect(Box<dyn ButtplugClientConnector>, ButtplugClientMessageStateShared),
    Disconnect,
    Message((ButtplugMessageUnion, ButtplugClientMessageStateShared)),
    NewClient(Sender<ButtplugMessageUnion>)
}

pub enum ButtplugInternalDeviceMessage {
    Message,
}

impl ButtplugClientInternalLoop {
    pub fn new(event_sender: Sender<ButtplugMessageUnion>) -> Self {
        let (cs, cr) = channel(256);
        ButtplugClientInternalLoop {
            connector: None,
            connected_devices: vec!(),
            connector_receiver: None,
            client_sender: cs,
            client_receiver: cr,
            event_sender: vec!(event_sender),
        }
    }

    pub fn get_client_sender(&self) -> Sender<ButtplugInternalClientMessage> {
        self.client_sender.clone()
    }

    pub async fn wait_for_event(&mut self) -> Option<ButtplugClientConnectorError> {

        info!("RUNNING INTERNAL LOOP");
        let mut r = None;
        if let Some(ref mut recv) = self.connector_receiver {
            //event_future = recv.clone().next().fuse();
            r = Some(recv.clone());
        }
        let mut event_recv = None;
        if let Some(ref mut re) = r {
            event_recv = Some(re.clone());
        }
        enum StreamReturn {
            ConnectorMessage(ButtplugMessageUnion),
            ClientMessage(ButtplugInternalClientMessage),
        }
        let mut client_receiver = self.client_receiver.clone();
        let client = task::spawn(async move {
            StreamReturn::ClientMessage(client_receiver.next().await.unwrap())
        });
        let mut stream_ret;
        if let Some(mut er) = event_recv {
            info!("Waiting on event and client!");
            let event = task::spawn(async move {
                StreamReturn::ConnectorMessage(er.next().await.unwrap())
            });
            stream_ret = select!(event, client).await;
        } else {
            info!("Waiting on client!");
            stream_ret = client.await;
        }
        match stream_ret {
            StreamReturn::ConnectorMessage(_msg) => {
                for ref mut sender in self.event_sender.iter() {
                    info!("Sending message to clients!");
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
                                info!("Connected!");
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
                        info!("Adding new client!");
                        self.event_sender.push(_sender);
                        None
                    },
                    _ => panic!("Message not handled!")
                }
            },
        }
    }
}

