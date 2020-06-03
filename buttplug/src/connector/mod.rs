// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Methods for establishing connections between Buttplug Clients and Servers
//!
//! Buttplug is made to work in many different circumstances. The
//! [ButtplugClient] and [ButtplugServer] may be in the same process, in
//! different process communicating over some sort of IPC, or on different
//! machines using a network connection. Connectors are what make these setups
//! possible.
//!
//! # How Buttplug Clients and Servers Use Connectors
//!
//! A Buttplug Client uses a connector to communicate with a server, be it in
//! the same process or on another machine. The client's connector handles
//! establishing the connection to the server, as well as sending ([possibly
//! serialized][crate::core::messages::serializer]) messages to the server and
//! matching replies from the server to waiting futures.
//!
//! Buttplug servers use connectors to receive info from clients. They usually
//! have less to do than client connectors, because they don't have to keep
//! track of messages waiting for replies (since Buttplug messages that require
//! responses are only client -> server, the server will never send anything to
//! a client that expects a response.)
//!
//! # In-Process and Remote Connectors
//!
//! There are two types of connectors: In-Process and Remote. All connectors
//! have the same API (since they all follow the [ButtplugClientConnector] and
//! [ButtplugServerConnector] traits), but will varying in latency, message
//! passing techniques, etc...
//!
//! There is only 1 in-process connector, the
//! [ButtplugInProcessClientConnector]. This is used when the client and server
//! live in the same process, which is useful for multiple reasons (see
//! [ButtplugInProcessClientConnector] documentation for more info). As
//! in-process connectors can just send message objects back and forth, there is
//! no need for message serialization.
//!
//! Remote connectors refer to any connector that connects to something outside
//! of the current process, be it still on the same machine (IPC) or somewhere
//! else (network).
//!
//! # Remote Transports
//!
//! Remote Transports
//!
//! # Buttplug Client/Server Does Not Necessarily Mean Transport Client/Server
//!
//! Here's an odd but valid situation: *You can have a Buttplug Client that uses
//! a Websocket Server connector!*
//!
//! There are times where this is actually useful. For instance, let's say a
//! user of Buttplug wants to use a Windows 7 desktop machine to control a
//! Bluetooth LE toy. This normally wouldn't work because Windows 7 doesn't have
//! a Bluetooth LE API we can easily access. However, they also have an android
//! phone. They could run a Buttplug Server in Chrome on their Android phone,
//! have the Client on the desktop run a websocket server, then have (and stick
//! with me here) the Buttplug Server in the Android Chrome instance use a
//! Websocket Client to connect to the Websocket Server on the desktop. This
//! allows the desktop machine to proxy Bluetooth to the WebBluetooth API built
//! into Android Chrome.
//!
//! Is this ridiculous? *Absolutely*.
//!
//! Will people do it? Remember, this is a library about sex, so the answer is
//! also *Absolutely*.
//!
//! There are slightly more useful situations like device forwarders where this
//! work comes in also, but that Windows 7/Android example is where the idea
//! originally came from.

mod in_process_connector;
mod remote_connector;
mod transport;

pub use in_process_connector::ButtplugInProcessClientConnector;
pub use remote_connector::{ButtplugRemoteClientConnector, ButtplugRemoteConnector};
pub use transport::ButtplugWebsocketClientTransport;

use crate::{
  core::messages::{serializer::ButtplugSerializedMessage, ButtplugMessage},
  util::future::{ButtplugFuture, ButtplugFutureState, ButtplugFutureStateShared},
};
use async_std::sync::{Receiver, Sender};
use async_trait::async_trait;
use std::{error::Error, fmt};
use futures::future::{self, BoxFuture};

pub type ButtplugConnectorResult = Result<(), ButtplugConnectorError>;
pub type ButtplugConnectorState = ButtplugFutureState<Result<(), ButtplugConnectorError>>;
pub type ButtplugConnectorStateShared =
  ButtplugFutureStateShared<Result<(), ButtplugConnectorError>>;
pub type ButtplugConnectorFuture = ButtplugFuture<Result<(), ButtplugConnectorError>>;
pub type ButtplugConnectorResultFuture = BoxFuture<'static, ButtplugConnectorResult>;

/// Errors specific to client connector structs.
///
/// Errors that relate to the communication method of the client connector. Can
/// include network/IPC protocol specific errors.
#[derive(Debug, Clone)]
pub struct ButtplugConnectorError {
  /// Error description
  pub message: String,
}

impl ButtplugConnectorError {
  /// Creates a new ButtplugClientConnectorError with a description.
  pub fn new(msg: &str) -> Self {
    Self {
      message: msg.to_owned(),
    }
  }
}

impl fmt::Display for ButtplugConnectorError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Init Error: {}", self.message)
  }
}

impl Error for ButtplugConnectorError {
  fn description(&self) -> &str {
    self.message.as_str()
  }

  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None
  }
}

impl<T> From<ButtplugConnectorError> for BoxFuture<'static, Result<T, ButtplugConnectorError>> where T: Send + 'static {
  fn from(err: ButtplugConnectorError) -> BoxFuture<'static, Result<T, ButtplugConnectorError>> {
    Box::pin(future::ready(Err(err)))
  }
}

/// Trait for client connectors.
///
/// Connectors are how Buttplug Clients and servers talk to each other. Whether
/// embedded, meaning the client and server exist in the same process space, or
/// remote, where the client and server are separated by some boundary, the
/// connector trait makes it so that these components always look local.
///
/// The `O` type specifies the outbound message type. This will usually be a
/// message enum. For instance, in a client connector, this would usually be
/// [ButtplugClientOutMessage][crate::core::messages::ButtplugClientOutMessage].
///
/// The `I` type specifies the inbound message type. This will usually be a
/// message enum. For instance, in a client connector, this would usually be
/// [ButtplugClientInMessage][crate::core::messages::ButtplugClientInMessage].
#[async_trait]
pub trait ButtplugConnector<OutboundMessageType, InboundMessageType>: Send + Sync
where
  OutboundMessageType: ButtplugMessage + 'static,
  InboundMessageType: ButtplugMessage + 'static,
{
  /// Connects the client to the server.
  ///
  /// Tries to connect to another connector, returning an event stream of
  /// incoming messages (of type `I`) on successful connect.
  ///
  /// # Errors
  ///
  /// Returns a [ButtplugClientConnectorError] if there is a problem with the
  /// connection process. It is assumed that all information needed to create
  /// the connection will be passed as part of the Trait implementors creation
  /// methods.
  ///
  /// # Async
  ///
  /// As connection may involve blocking operations like establishing network
  /// connections, this trait method is marked async.
  fn connect(&mut self) -> BoxFuture<'static, Result<Receiver<InboundMessageType>, ButtplugConnectorError>>;
  /// Disconnects the client from the server.
  ///
  /// Returns a [ButtplugClientConnectorError] if there is a problem with the
  /// disconnection process.
  ///
  /// # Async
  ///
  /// As disconnection may involve blocking operations like network closing and
  /// cleanup, this trait method is marked async.
  fn disconnect(&self) -> ButtplugConnectorResultFuture;
  /// Sends a message of outbound message type `O` to the other connector.
  ///
  /// # Errors
  ///
  /// If the connector is not currently connected, or an error happens during
  /// the send operation, this will return a [ButtplugConnectorError]
  fn send(&self, msg: OutboundMessageType) -> ButtplugConnectorResultFuture;
}

/// Enum of messages we can receive from a connector.
pub enum ButtplugTransportMessage {
  /// Send when connection is established.
  Connected,
  /// Text version of message we received from remote server.
  Message(ButtplugSerializedMessage),
  /// Error received from remote server.
  Error(String),
  /// Connector (or remote server) itself closed the connection.
  Close(String),
}

#[async_trait]
pub trait ButtplugConnectorTransport: Send + Sync {
  fn connect(
    &self,
  ) -> BoxFuture<'static, Result<
    (
      Sender<ButtplugSerializedMessage>,
      Receiver<ButtplugTransportMessage>,
    ),
    ButtplugConnectorError,
  >>;
  fn disconnect(self) -> ButtplugConnectorResultFuture;
}
