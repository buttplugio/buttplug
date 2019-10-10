use std::error::Error;
use std::fmt;
use async_trait::async_trait;
use futures::select;
use futures::future::Future;
use futures::{FutureExt, StreamExt, SinkExt};
use futures_channel::mpsc;
use super::client::ButtplugClientError;
use crate::core::messages::ButtplugMessageUnion;
use crate::server::server::ButtplugServer;
use super::messagesorter::{ClientConnectorMessageSorter, ClientConnectorMessageFuture};

#[derive(Debug, Clone)]
pub struct ButtplugClientConnectorError {
    pub message: String,
}

impl ButtplugClientConnectorError {
    pub fn new(msg: &str) -> ButtplugClientConnectorError {
        ButtplugClientConnectorError {
            message: msg.to_owned()
        }
    }
}

impl fmt::Display for ButtplugClientConnectorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Init Error: {}", self.message)
    }
}

impl Error for ButtplugClientConnectorError {
    fn description(&self) -> &str {
        self.message.as_str()
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[async_trait]
pub trait ButtplugClientConnector {
    async fn connect(&mut self) -> Option<ButtplugClientConnectorError>;
    fn disconnect(&mut self) -> Option<ButtplugClientConnectorError>;
    async fn send(&mut self, msg: &ButtplugMessageUnion) -> Result<ButtplugMessageUnion, ButtplugClientError>;
}

pub trait ButtplugRemoteClientConnectorSender: Sync + Send {
    fn send(&self, msg: ButtplugMessageUnion);
    fn close(&self);
}

pub enum ButtplugRemoteClientConnectorMessage {
    Sender(Box<dyn ButtplugRemoteClientConnectorSender>),
    Connected(),
    Text(String),
    Error(String),
    Close(String)
}

pub struct ButtplugRemoteClientConnectorHelper {
    // Channel send/recv pair for applications wanting to send out through the
    // remote connection. Receiver will be send to task on creation.
    internal_send: mpsc::UnboundedSender<ButtplugMessageUnion>,
    internal_recv: Option<mpsc::UnboundedReceiver<ButtplugMessageUnion>>,
    // Channel send/recv pair for remote connection sending information to the
    // application. Receiver will be send to task on creation.
    remote_send: mpsc::UnboundedSender<ButtplugRemoteClientConnectorMessage>,
    remote_recv: Option<mpsc::UnboundedReceiver<ButtplugRemoteClientConnectorMessage>>,
    // Channel receiver for getting futures back on the main thread from the
    // sorter (which lives in a future wherever the scheduler put it) when we
    // expect them.
    future_recv: Option<mpsc::UnboundedReceiver<ClientConnectorMessageFuture>>,
}

unsafe impl Send for ButtplugRemoteClientConnectorHelper {}
unsafe impl Sync for ButtplugRemoteClientConnectorHelper {}

impl ButtplugRemoteClientConnectorHelper {
    pub fn new() -> ButtplugRemoteClientConnectorHelper {
        let (internal_send, internal_recv) = mpsc::unbounded();
        let (remote_send, remote_recv) = mpsc::unbounded();
        ButtplugRemoteClientConnectorHelper {
            remote_send,
            remote_recv: Some(remote_recv),
            internal_send,
            internal_recv: Some(internal_recv),
            future_recv: None,
        }
    }

    pub fn get_internal_send(&self) -> mpsc::UnboundedSender<ButtplugMessageUnion> {
        self.internal_send.clone()
    }

    pub fn get_remote_send(&self) -> mpsc::UnboundedSender<ButtplugRemoteClientConnectorMessage> {
        self.remote_send.clone()
    }

    pub async fn send(&mut self, msg: &ButtplugMessageUnion) -> Result<ButtplugMessageUnion, ButtplugClientError> {
        if let Some(ref mut fr) = self.future_recv {
            self.internal_send.send(msg.clone()).await;
            let fut = fr.next().await;
            Ok(fut.unwrap().await)
        } else {
            Err(ButtplugClientError::ButtplugClientConnectorError(ButtplugClientConnectorError::new("Do not have receiver yet")))
        }
    }

    pub fn get_recv_future(&mut self) -> impl Future {
        // Set up a way to get futures in and out of the sorter, which will live
        // in our connector task.
        let (mut future_send, future_recv) = mpsc::unbounded::<ClientConnectorMessageFuture>();
        self.future_recv = Some(future_recv);

        // Remove the receivers we need to move into the task.
        let mut remote_recv = self.remote_recv.take().unwrap();
        let mut internal_recv = self.internal_recv.take().unwrap();
        async move {
            let mut sorter = ClientConnectorMessageSorter::new();
            // Our in-task remote sender, which is a wrapped version of whatever
            // bus specific sender (websocket, tcp, etc) we'll be using.
            let mut remote_send: Option<Box<dyn ButtplugRemoteClientConnectorSender>> = None;

            enum StreamValue {
                NoValue,
                Incoming(ButtplugRemoteClientConnectorMessage),
                Outgoing(ButtplugMessageUnion)
            }

            loop {
                let mut incoming_stream = remote_recv.next().fuse();
                let mut outgoing_stream = internal_recv.next().fuse();
                // We use two Options instead of an enum because we may never
                // get anything.
                let mut stream_return: StreamValue = select! {
                    a = incoming_stream => {
                        match a {
                            Some(msg) => StreamValue::Incoming(msg),
                            None => StreamValue::NoValue,
                        }
                    },
                    b = outgoing_stream => {
                        match b {
                            Some(msg) => StreamValue::Outgoing(msg),
                            None => StreamValue::NoValue,
                        }
                    },
                };
                match stream_return {
                    StreamValue::NoValue => break,
                    StreamValue::Incoming(remote_msg) => {
                        match remote_msg {
                            ButtplugRemoteClientConnectorMessage::Sender(_s) => {
                                remote_send = Some(_s);
                            },
                            ButtplugRemoteClientConnectorMessage::Text(_t) => {
                                let array: Vec<ButtplugMessageUnion> = serde_json::from_str(&_t.clone()).unwrap();
                                for smsg in array {
                                    if !sorter.resolve_message(&smsg) {
                                        // TODO Fill this in with notifications.
                                    }
                                }
                            }
                            _ => {
                                panic!("UNHANDLED BRANCH");
                            }
                        }
                    },
                    StreamValue::Outgoing(ref mut buttplug_msg) => {
                        if let Some(ref mut remote_sender) = remote_send {
                            remote_sender.send(buttplug_msg.clone());
                        }
                        let f = sorter.create_future(buttplug_msg);
                        if future_send.send(f).await.is_err() {
                            println!("SEND ERR");
                        }
                    }
                }
            }
        }
    }
}

pub struct ButtplugEmbeddedClientConnector {
    server: ButtplugServer,
}

impl ButtplugEmbeddedClientConnector {
    pub fn new(name: &str, max_ping_time: u32) -> ButtplugEmbeddedClientConnector {
        ButtplugEmbeddedClientConnector {
            server: ButtplugServer::new(&name, max_ping_time),
        }
    }
}

#[async_trait]
impl ButtplugClientConnector for ButtplugEmbeddedClientConnector {
    async fn connect(&mut self) -> Option<ButtplugClientConnectorError> {
        None
    }

    fn disconnect(&mut self) -> Option<ButtplugClientConnectorError> {
        None
    }

    async fn send(&mut self, msg: &ButtplugMessageUnion) -> Result<ButtplugMessageUnion, ButtplugClientError> {
        self.server
            .send_message(msg)
            .await
            .map_err(|x| ButtplugClientError::ButtplugError(x))
    }
}

// The embedded connector is used heavily in the client unit tests, so we can
// assume code coverage there and omit specific tests here.

