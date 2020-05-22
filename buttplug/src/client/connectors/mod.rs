// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug Client Connectors, for communicating with Buttplug Servers
mod message_sorter;
#[cfg(feature = "serialize_json")]
mod remote_connector_helper;
#[cfg(any(feature = "client-ws", feature = "client-ws-ssl"))]
pub mod websocket;

#[cfg(feature = "serialize_json")]
pub use message_sorter::ClientConnectorMessageSorter;
#[cfg(feature = "serialize_json")]
pub use remote_connector_helper::{ButtplugRemoteClientConnectorHelper, ButtplugRemoteClientConnectorMessage};

use super::{
  ButtplugClientMessageFuture,
  ButtplugClientMessageFuturePair,
  ButtplugInternalClientMessageResult,
};
#[cfg(feature = "server")]
use crate::server::{ButtplugInProcessServerWrapper, ButtplugServer, ButtplugServerWrapper};
use crate::{
  core::{
    messages::{
      ButtplugClientInMessage,
      ButtplugClientOutMessage,
    },
  },
  util::future::{ButtplugFuture, ButtplugFutureState, ButtplugFutureStateShared},
};
use async_std::sync::Receiver;
#[cfg(feature = "serialize_json")]
use async_std::prelude::StreamExt;
use async_trait::async_trait;
use std::{error::Error, fmt};

pub type ButtplugClientConnectorResult = Result<(), ButtplugClientConnectorError>;
pub type ButtplugClientConnectorState = ButtplugFutureState<ButtplugClientConnectorResult>;
pub type ButtplugClientConnectorStateShared =
  ButtplugFutureStateShared<ButtplugClientConnectorResult>;
pub type ButtplugClientConnectorFuture = ButtplugFuture<ButtplugClientConnectorResult>;

/// Errors specific to client connector structs.
///
/// Errors that relate to the communication method of the client connector. Can
/// include network/IPC protocol specific errors.
#[derive(Debug, Clone)]
pub struct ButtplugClientConnectorError {
  /// Error description
  pub message: String,
}

impl ButtplugClientConnectorError {
  /// Creates a new ButtplugClientConnectorError with a description.
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

/// Trait for client connectors.
///
/// Connectors are how Buttplug Clients talk to Buttplug Servers. Whether
/// embedded, meaning the client and server exist in the same process space, or
/// remote, where the client and server are separated by some boundary, the
/// connector trait makes it so that the client does not need to be aware of the
/// specifics of where the server is.
///
#[async_trait]
pub trait ButtplugClientConnector: Send + Sync {
  /// Connects the client to the server.
  ///
  /// Returns a [ButtplugClientConnectorError] if there is a problem with the
  /// connection process. It is assumed that all information needed to create
  /// the connection will be passed as part of the Trait implementors creation
  /// methods.
  ///
  /// As connection may involve blocking operations (like network connections),
  /// this trait method is marked async.
  async fn connect(&mut self) -> ButtplugClientConnectorResult;
  /// Disconnects the client from the server.
  ///
  /// Returns a [ButtplugClientConnectorError] if there is a problem with the
  /// disconnection process.
  ///
  /// As disconnection may involve blocking operations (like network closing and
  /// cleanup), this trait method is marked async.
  async fn disconnect(&mut self) -> ButtplugClientConnectorResult;
  /// Sends a
  /// [ButtplugClientInMessage][crate::core::messages::ButtplugClientInMessage]
  /// to the server.
  async fn send(&mut self, msg: ButtplugClientInMessage) -> ButtplugInternalClientMessageResult;
  /// Takes the event receiver from the connector.
  ///
  /// # Panics
  ///
  /// Will panic if called twice.
  // TODO Should probably just return a result that has an error, versus panicing? This is recoverable.
  fn get_event_receiver(&mut self) -> Receiver<ButtplugClientOutMessage>;
}

/// In-process Buttplug Server Connector
///
/// The Embedded Server contains a [ButtplugServer], meaning that both the
/// [ButtplugClient] and [ButtplugServer] will exist in the same process. This
/// is useful for developing applications, or for distributing an applications
/// without requiring access to an outside [ButtplugServer].
///
/// # Notes
///
/// Buttplug, as a project, is built in a way that tries to make sure all
/// programs will work with new versions of the library. This is why we have
/// [ButtplugClient] for applications, and Connectors to access out-of-process
/// [ButtplugServer]s over IPC, network, etc. It means that the out-of-process
/// server can be upgraded by the user at any time, even if the [ButtplugClient]
/// using application hasn't been upgraded. This allows the program to support
/// hardware that may not have even been released when it was published.
///
/// While including an EmbeddedConnector in your application is the quickest and
/// easiest way to develop (and we highly recommend developing that way), and
/// also an easy way to get users up and running as quickly as possible, we
/// recommend also including some sort of IPC Connector in order for your
/// application to connect to newer servers when they come out.
#[cfg(feature = "server")]
pub struct ButtplugEmbeddedClientConnector {
  /// Internal server object for the embedded connector.
  server: ButtplugInProcessServerWrapper,
  /// Event receiver for the internal server.
  recv: Option<Receiver<ButtplugClientOutMessage>>,
}

#[cfg(feature = "server")]
impl<'a> ButtplugEmbeddedClientConnector {
  /// Creates a new embedded connector, with a server instance.
  ///
  /// Sets up a server, using the basic [ButtplugServer] construction arguments.
  /// Takes the server's name and the ping time it should use, with a ping time
  /// of 0 meaning infinite ping.
  pub fn new(name: &str, max_ping_time: u128) -> Self {
    let (server, recv) = ButtplugInProcessServerWrapper::new(&name, max_ping_time);
    Self {
      recv: Some(recv),
      server,
    }
  }

  /// Get a reference to the internal server.
  ///
  /// Allows the owner to manipulate the internal server instance. Useful for
  /// setting up [DeviceCommunicationManager]s before connection.
  pub fn server_ref(&'a mut self) -> &'a mut ButtplugServer {
    self.server.server_ref()
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

  async fn send(
    &mut self,
    msg: ButtplugClientInMessage,
  ) -> Result<ButtplugClientOutMessage, ButtplugClientConnectorError> {
    Ok(self.server.parse_message(msg).await)
  }

  fn get_event_receiver(&mut self) -> Receiver<ButtplugClientOutMessage> {
    // This will panic if we've already taken the receiver.
    self.recv.take().unwrap()
  }
}

// The embedded connector is used heavily in the client unit tests, so we can
// assume code coverage there and omit specific tests here.
