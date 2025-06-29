// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

pub mod stream;

use crate::connector::{
  ButtplugConnectorError,
  ButtplugConnectorResultFuture,
  ButtplugSerializedMessage,
};
use displaydoc::Display;
use futures::future::BoxFuture;
use thiserror::Error;
use tokio::sync::mpsc::{Receiver, Sender};

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
  #[error("Network error: {0}")]
  GenericNetworkError(String),
}
