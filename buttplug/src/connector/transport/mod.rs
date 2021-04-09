#[cfg(feature = "websockets")]
mod websocket;
use crate::connector::{
  ButtplugConnectorError,
  ButtplugConnectorResultFuture,
  ButtplugSerializedMessage,
};
use futures::future::BoxFuture;
use tokio::sync::mpsc::{Receiver, Sender};
#[cfg(feature = "websockets")]
pub use websocket::{ButtplugWebsocketClientTransport, TungsteniteError};
#[cfg(feature = "websockets")]
pub use websocket::{ButtplugWebsocketServerTransport, ButtplugWebsocketServerTransportOptions};

use thiserror::Error;

/// Messages we can receive from a connector.
#[derive(Clone, Debug, Display)]
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

pub trait ButtplugConnectorTransport: Send + Sync {
  fn connect(
    &self,
    outgoing_receiver: Receiver<ButtplugSerializedMessage>,
    incoming_sender: Sender<ButtplugTransportIncomingMessage>,
  ) -> BoxFuture<'static, Result<(), ButtplugConnectorError>>;
  fn disconnect(self) -> ButtplugConnectorResultFuture;
}

#[derive(Error, Debug)]
pub enum ButtplugConnectorTransportSpecificError {
  #[cfg(feature = "websockets")]
  #[error("Tungstenite specific error: {0}")]
  TungsteniteError(#[from] TungsteniteError),
}
