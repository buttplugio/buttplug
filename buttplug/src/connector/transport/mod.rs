mod websocket_transport;

use crate::connector::{
  ButtplugConnectorError, ButtplugConnectorResultFuture, ButtplugSerializedMessage,
};
use async_channel::{Receiver, Sender};
use futures::future::BoxFuture;
pub use websocket_transport::ButtplugWebsocketClientTransport;

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
