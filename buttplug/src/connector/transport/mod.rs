#[cfg(feature = "websockets")]
mod websocket;
use crate::connector::{
  ButtplugConnectorError,
  ButtplugConnectorResultFuture,
  ButtplugSerializedMessage,
};
use async_channel::{Receiver, Sender};
use futures::future::BoxFuture;
#[cfg(feature = "websockets")]
pub use websocket::{ButtplugWebsocketClientTransport, TungsteniteError};
#[cfg(all(feature = "websockets", feature = "async-std-runtime"))]
pub use websocket::{ButtplugWebsocketServerTransport, ButtplugWebsocketServerTransportOptions};

use thiserror::Error;

/// Messages we can send thru the connector.
pub enum ButtplugTransportOutgoingMessage {
  /// Text version of message we are sending to the remote server.
  Message(ButtplugSerializedMessage),
  /// Request for connector to close the connection
  Close,
}

/// Messages we can receive from a connector.
pub enum ButtplugTransportIncomingMessage {
  /// Send when connection is established.
  Connected,
  /// Serialized version of message we received from remote server.
  Message(ButtplugSerializedMessage),
  // TODO Implement binary message at some point.
  /// Error received from remote server.
  Error(String),
  /// Connector (or remote server) itself closed the connection.
  Close(String),
}

pub type ButtplugConnectorTransportConnectResult = BoxFuture<
  'static,
  Result<
    (
      Sender<ButtplugTransportOutgoingMessage>,
      Receiver<ButtplugTransportIncomingMessage>,
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
  #[cfg(feature = "websockets")]
  #[error("Tungstenite specific error: {0}")]
  TungsteniteError(#[from] TungsteniteError),

  #[error("Secure server error: %s")]
  SecureServerError(String),
}
