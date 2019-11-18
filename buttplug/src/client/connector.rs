// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handling of communication with Buttplug Server.

use super::{
    internal::{
        ButtplugClientFuture, ButtplugClientFutureState, ButtplugClientFutureStateShared,
        ButtplugClientMessageStateShared,
    },
    messagesorter::ClientConnectorMessageSorter,
};
use crate::{core::messages::ButtplugMessageUnion, server::ButtplugServer};
use async_std::{
    prelude::{FutureExt, StreamExt},
    sync::{channel, Receiver, Sender},
};
use async_trait::async_trait;
use futures::future::Future;
use std::{error::Error, fmt};

pub type ButtplugClientConnectionState =
    ButtplugClientFutureState<Option<ButtplugClientConnectorError>>;
pub type ButtplugClientConnectionStateShared =
    ButtplugClientFutureStateShared<Option<ButtplugClientConnectorError>>;
pub type ButtplugClientConnectionFuture =
    ButtplugClientFuture<Option<ButtplugClientConnectorError>>;

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
    async fn connect(&mut self) -> Option<ButtplugClientConnectorError>;
    async fn disconnect(&mut self) -> Option<ButtplugClientConnectorError>;
    async fn send(&mut self, msg: &ButtplugMessageUnion, state: &ButtplugClientMessageStateShared);
    fn get_event_receiver(&mut self) -> Receiver<ButtplugMessageUnion>;
}

pub struct ButtplugEmbeddedClientConnector {
    server: ButtplugServer,
    recv: Option<Receiver<ButtplugMessageUnion>>,
}

impl ButtplugEmbeddedClientConnector {
    pub fn new(name: &str, max_ping_time: u32) -> Self {
        let (send, recv) = channel(256);
        Self {
            recv: Some(recv),
            server: ButtplugServer::new(&name, max_ping_time, send),
        }
    }
}

#[async_trait]
impl ButtplugClientConnector for ButtplugEmbeddedClientConnector {
    async fn connect(&mut self) -> Option<ButtplugClientConnectorError> {
        None
    }

    async fn disconnect(&mut self) -> Option<ButtplugClientConnectorError> {
        None
    }

    async fn send(&mut self, msg: &ButtplugMessageUnion, state: &ButtplugClientMessageStateShared) {
        let ret_msg = self.server.send_message(msg).await;
        let mut waker_state = state.lock().unwrap();
        waker_state.set_reply(ret_msg.unwrap());
    }

    fn get_event_receiver(&mut self) -> Receiver<ButtplugMessageUnion> {
        // This will panic if we've already taken the receiver.
        self.recv.take().unwrap()
    }
}

// The embedded connector is used heavily in the client unit tests, so we can
// assume code coverage there and omit specific tests here.

pub trait ButtplugRemoteClientConnectorSender: Sync + Send {
    fn send(&self, msg: ButtplugMessageUnion);
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

pub struct ButtplugRemoteClientConnectorHelper {
    // Channel send/recv pair for applications wanting to send out through the
    // remote connection. Receiver will be send to task on creation.
    internal_send: Sender<(ButtplugMessageUnion, ButtplugClientMessageStateShared)>,
    internal_recv: Option<Receiver<(ButtplugMessageUnion, ButtplugClientMessageStateShared)>>,
    // Channel send/recv pair for remote connection sending information to the
    // application. Receiver will be sent to task on creation.
    remote_send: Sender<ButtplugRemoteClientConnectorMessage>,
    remote_recv: Option<Receiver<ButtplugRemoteClientConnectorMessage>>,
    event_send: Option<Sender<ButtplugMessageUnion>>,
}

unsafe impl Send for ButtplugRemoteClientConnectorHelper {}
unsafe impl Sync for ButtplugRemoteClientConnectorHelper {}

impl ButtplugRemoteClientConnectorHelper {
    pub fn new(event_sender: Sender<ButtplugMessageUnion>) -> Self {
        let (internal_send, internal_recv) = channel(256);
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

    pub async fn send(
        &mut self,
        msg: &ButtplugMessageUnion,
        state: &ButtplugClientMessageStateShared,
    ) {
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
            let mut sorter = ClientConnectorMessageSorter::new();
            // Our in-task remote sender, which is a wrapped version of whatever
            // bus specific sender (websocket, tcp, etc) we'll be using.
            let mut remote_send: Option<Box<dyn ButtplugRemoteClientConnectorSender>> = None;

            enum StreamValue {
                NoValue,
                Incoming(ButtplugRemoteClientConnectorMessage),
                Outgoing((ButtplugMessageUnion, ButtplugClientMessageStateShared)),
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
                                let array: Vec<ButtplugMessageUnion> =
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
