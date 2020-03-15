// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handling of communication with Buttplug Server.
pub mod messagesorter;
#[cfg(any(feature = "client-ws", feature = "client-ws-ssl"))]
pub mod websocket;

use crate::server::device_manager::{
    DeviceCommunicationManager, DeviceCommunicationManagerCreator,
};
#[cfg(feature = "server")]
use crate::server::{ButtplugInProcessServerWrapper, ButtplugServerWrapper};
use crate::{
    core::messages::{ButtplugClientInMessage, ButtplugClientOutMessage},
    util::future::{
        ButtplugFuture, ButtplugFutureState, ButtplugFutureStateShared, ButtplugMessageStateShared,
    },
};
use async_std::sync::{channel, Receiver};
#[cfg(feature = "serialize_json")]
use async_std::{
    prelude::{FutureExt, StreamExt},
    sync::Sender,
};
use async_trait::async_trait;
#[cfg(feature = "serialize_json")]
use futures::future::Future;
#[cfg(feature = "serialize_json")]
use messagesorter::ClientConnectorMessageSorter;
use std::{error::Error, fmt};

pub type ButtplugClientConnectionState =
    ButtplugFutureState<Result<(), ButtplugClientConnectorError>>;
pub type ButtplugClientConnectionStateShared =
    ButtplugFutureStateShared<Result<(), ButtplugClientConnectorError>>;
pub type ButtplugClientConnectionFuture = ButtplugFuture<Result<(), ButtplugClientConnectorError>>;

#[derive(Debug, Clone)]
pub struct ButtplugClientConnectorError {
    pub message: String,
}

impl ButtplugClientConnectorError {
    pub fn new(msg: &str) -> Self {
        Self {
            message: msg.to_owned(),
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

// Not real sure if this is sync, since there may be state that could get weird
// in connectors implementing this trait, but Send should be ok.
#[async_trait]
pub trait ButtplugClientConnector: Send {
    async fn connect(&mut self) -> Result<(), ButtplugClientConnectorError>;
    async fn disconnect(&mut self) -> Result<(), ButtplugClientConnectorError>;
    async fn send(&mut self, msg: ButtplugClientInMessage, state: &ButtplugMessageStateShared);
    fn get_event_receiver(&mut self) -> Receiver<ButtplugClientOutMessage>;
}

#[cfg(feature = "server")]
pub struct ButtplugEmbeddedClientConnector {
    server: ButtplugInProcessServerWrapper,
    recv: Option<Receiver<ButtplugClientOutMessage>>,
}

#[cfg(feature = "server")]
impl ButtplugEmbeddedClientConnector {
    pub fn new(name: &str, max_ping_time: u128) -> Self {
        let (server, recv) = ButtplugInProcessServerWrapper::new(&name, max_ping_time);
        Self {
            recv: Some(recv),
            server
        }
    }

    // TODO Is there some way to do this on the server then pass the server in,
    // versus just adding through the connector? This feels a little weird.
    pub fn add_comm_manager<T>(&mut self)
    where
        T: 'static + DeviceCommunicationManager + DeviceCommunicationManagerCreator,
    {
        self.server.server_ref().add_comm_manager::<T>();
    }
}

#[cfg(feature = "server")]
#[async_trait]
impl ButtplugClientConnector for ButtplugEmbeddedClientConnector {
    async fn connect(&mut self) -> Result<(), ButtplugClientConnectorError> {
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), ButtplugClientConnectorError> {
        Ok(())
    }

    async fn send(&mut self, msg: ButtplugClientInMessage, state: &ButtplugMessageStateShared) {
        let ret_msg = self.server.parse_message(msg).await;
        let mut waker_state = state.lock().unwrap();
        waker_state.set_reply(ret_msg);
    }

    fn get_event_receiver(&mut self) -> Receiver<ButtplugClientOutMessage> {
        // This will panic if we've already taken the receiver.
        self.recv.take().unwrap()
    }
}

// The embedded connector is used heavily in the client unit tests, so we can
// assume code coverage there and omit specific tests here.

pub trait ButtplugRemoteClientConnectorSender: Sync + Send {
    fn send(&self, msg: ButtplugClientInMessage);
    fn close(&self);
}

pub enum ButtplugRemoteClientConnectorMessage {
    Sender(Box<dyn ButtplugRemoteClientConnectorSender>),
    Connected(),
    Text(String),
    Error(String),
    ClientClose(String),
    Close(String),
}

#[cfg(feature = "serialize_json")]
pub struct ButtplugRemoteClientConnectorHelper {
    // Channel send/recv pair for applications wanting to send out through the
    // remote connection. Receiver will be send to task on creation.
    internal_send: Sender<(ButtplugClientInMessage, ButtplugMessageStateShared)>,
    internal_recv: Option<Receiver<(ButtplugClientInMessage, ButtplugMessageStateShared)>>,
    // Channel send/recv pair for remote connection sending information to the
    // application. Receiver will be sent to task on creation.
    remote_send: Sender<ButtplugRemoteClientConnectorMessage>,
    remote_recv: Option<Receiver<ButtplugRemoteClientConnectorMessage>>,
    event_send: Option<Sender<ButtplugClientOutMessage>>,
}

#[cfg(feature = "serialize_json")]
unsafe impl Send for ButtplugRemoteClientConnectorHelper {}
#[cfg(feature = "serialize_json")]
unsafe impl Sync for ButtplugRemoteClientConnectorHelper {}

#[cfg(feature = "serialize_json")]
impl ButtplugRemoteClientConnectorHelper {
    pub fn new(event_sender: Sender<ButtplugClientOutMessage>) -> Self {
        let (internal_send, internal_recv) = channel::<(ButtplugClientInMessage, ButtplugMessageStateShared)>(256);
        let (remote_send, remote_recv) = channel(256);
        Self {
            event_send: Some(event_sender),
            remote_send,
            remote_recv: Some(remote_recv),
            internal_send,
            internal_recv: Some(internal_recv),
        }
    }

    pub fn get_remote_send(&self) -> Sender<ButtplugRemoteClientConnectorMessage> {
        self.remote_send.clone()
    }

    pub async fn send(&mut self, msg: &ButtplugClientInMessage, state: &ButtplugMessageStateShared) {
        self.internal_send.send((msg.clone(), state.clone())).await;
    }

    pub async fn close(&self) {
        // Emulate a close from the connector side, which will cause us to
        // close.
        self.remote_send
            .send(ButtplugRemoteClientConnectorMessage::Close(
                "Client requested close.".to_owned(),
            ))
            .await;
    }

    pub fn get_recv_future(&mut self) -> impl Future {
        // Set up a way to get futures in and out of the sorter, which will live
        // in our connector task.
        let event_send = self.event_send.take().unwrap();

        // Remove the receivers we need to move into the task.
        let mut remote_recv = self.remote_recv.take().unwrap();
        let mut internal_recv = self.internal_recv.take().unwrap();
        async move {
            let mut sorter = ClientConnectorMessageSorter::default();
            // Our in-task remote sender, which is a wrapped version of whatever
            // bus specific sender (websocket, tcp, etc) we'll be using.
            let mut remote_send: Option<Box<dyn ButtplugRemoteClientConnectorSender>> = None;

            enum StreamValue {
                NoValue,
                Incoming(ButtplugRemoteClientConnectorMessage),
                Outgoing((ButtplugClientInMessage, ButtplugMessageStateShared)),
            }

            loop {
                // We use two Options instead of an enum because we may never
                // get anything.
                let mut stream_return: StreamValue = async {
                    match remote_recv.next().await {
                        Some(msg) => StreamValue::Incoming(msg),
                        None => StreamValue::NoValue,
                    }
                }
                .race(async {
                    match internal_recv.next().await {
                        Some(msg) => StreamValue::Outgoing(msg),
                        None => StreamValue::NoValue,
                    }
                })
                .await;
                match stream_return {
                    StreamValue::NoValue => break,
                    StreamValue::Incoming(remote_msg) => {
                        match remote_msg {
                            ButtplugRemoteClientConnectorMessage::Sender(s) => {
                                remote_send = Some(s);
                            }
                            ButtplugRemoteClientConnectorMessage::Text(t) => {
                                let array: Vec<ButtplugClientOutMessage> =
                                    serde_json::from_str(&t.clone()).unwrap();
                                for smsg in array {
                                    if !sorter.maybe_resolve_message(&smsg) {
                                        info!("Sending event!");
                                        // Send notification through event channel
                                        event_send.send(smsg).await;
                                    }
                                }
                            }
                            ButtplugRemoteClientConnectorMessage::ClientClose(s) => {
                                info!("Client closing connection {}", s);
                                if let Some(ref mut remote_sender) = remote_send {
                                    remote_sender.close();
                                } else {
                                    panic!("Can't send message yet!");
                                }
                            }
                            ButtplugRemoteClientConnectorMessage::Close(s) => {
                                info!("Connector closing connection {}", s);
                                break;
                            }
                            _ => {
                                panic!("UNHANDLED BRANCH");
                            }
                        }
                    }
                    StreamValue::Outgoing(ref mut buttplug_fut_msg) => {
                        // Create future sets our message ID, so make sure this
                        // happens before we send out the message.
                        sorter.register_future(&mut buttplug_fut_msg.0, &buttplug_fut_msg.1);
                        if let Some(ref mut remote_sender) = remote_send {
                            remote_sender.send(buttplug_fut_msg.0.clone());
                        } else {
                            panic!("Can't send message yet!");
                        }
                    }
                }
            }
        }
    }
}
