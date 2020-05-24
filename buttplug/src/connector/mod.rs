mod util;
mod in_process;
mod websocket;

pub use in_process::{ButtplugInProcessClientConnector, ButtplugInProcessServerConnector};
pub use websocket::ButtplugWebsocketClientConnector;
pub use util::{ClientConnectorMessageSorter, ButtplugRemoteClientConnectorHelper, ButtplugRemoteClientConnectorMessage};

use crate::server::ButtplugServer;
use async_trait::async_trait;
use async_std::sync::Receiver;
use std::{fmt, error::Error};
use crate::{
  core::messages::{ButtplugClientInMessage, ButtplugClientOutMessage},
  client::{ButtplugInternalClientMessageResult},
  util::future::{ButtplugFutureState, ButtplugFuture, ButtplugFutureStateShared}
};


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
  // TODO Return receiver on connect?
  fn get_event_receiver(&mut self) -> Receiver<ButtplugClientOutMessage>;
}

#[async_trait]
pub trait ButtplugServerConnector {
  type Input;
  type Output;

  async fn parse_message(&mut self, msg: Self::Input) -> Self::Output;
  fn server_ref(&mut self) -> &mut ButtplugServer;
}