// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Transports for remote (IPC/network/etc) communication between clients and servers

#[cfg(feature = "websockets")]
mod websocket;
use crate::core::connector::{
  ButtplugConnectorError,
  ButtplugConnectorResultFuture,
  ButtplugSerializedMessage,
};
use futures::future::BoxFuture;
use thiserror::Error;
use tokio::sync::mpsc::{Receiver, Sender};
#[cfg(feature = "websockets")]
pub use websocket::{
  ButtplugWebsocketClientTransport,
  ButtplugWebsocketServerTransport,
  ButtplugWebsocketServerTransportBuilder,
  TungsteniteError,
};

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
  #[error("Network error: {0}")]
  GenericNetworkError(String),
}
