#[cfg(feature="client-ws")]
mod websocket;

use crate::connector::{
  ButtplugConnectorError, ButtplugConnectorResultFuture, ButtplugSerializedMessage,
};
use async_channel::{Receiver, Sender};
use futures::future::BoxFuture;
#[cfg(feature="client-ws")]
pub use websocket::{ButtplugWebsocketClientTransport, ButtplugWebsocketServerTransport, ButtplugWebsocketServerTransportOptions};
#[cfg(feature="client-ws")]
use async_tungstenite::tungstenite::Error as TungsteniteError;
use thiserror::Error;

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

pub type ButtplugConnectorTransportConnectResult = BoxFuture<
  'static,
  Result<
    (
      Sender<ButtplugSerializedMessage>,
      Receiver<ButtplugTransportMessage>,
    ),
    ButtplugConnectorError,
  >,
>;

pub trait ButtplugConnectorTransport: Send + Sync {
  fn connect(&self) -> ButtplugConnectorTransportConnectResult;
  fn disconnect(self) -> ButtplugConnectorResultFuture;
}

#[derive(Error, Debug)]
pub enum ButtplugConnectorTransportSpecificError {
  #[cfg(feature="client-ws")]
  #[error("Tungstenite specific error: {0}")]
  TungsteniteError(#[from] TungsteniteError),

  #[error("Secure server error: %s")]
  SecureServerError(String)
}