// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2021 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handling of named pipes and unix domain sockets, via tokio.

use crate::{
  connector::{
    transport::{
      ButtplugConnectorTransport,
      ButtplugConnectorTransportSpecificError,
      ButtplugTransportIncomingMessage,
    },
    ButtplugConnectorError,
    ButtplugConnectorResultFuture,
  },
  core::messages::serializer::ButtplugSerializedMessage,
};
use futures::future::BoxFuture;
use std::sync::Arc;
#[cfg(target_os = "windows")]
use tokio::net::windows::named_pipe;
#[cfg(not(target_os = "windows"))]
use tokio::net::UnixStream;
use tokio::{
  io::{AsyncWriteExt, Interest},
  sync::{
    mpsc::{Receiver, Sender},
    Notify,
  },
};

#[cfg(target_os = "windows")]
type PipeClientType = named_pipe::NamedPipeClient;
#[cfg(not(target_os = "windows"))]
type PipeClientType = UnixStream;

#[derive(Clone, Debug)]
pub struct ButtplugPipeClientTransportBuilder {
  /// Address (either Named Pipe or Domain Socket File) to connect to
  address: String,
}

impl ButtplugPipeClientTransportBuilder {
  pub fn new(address: &str) -> Self {
    Self {
      address: address.to_owned(),
    }
  }

  pub fn finish(self) -> ButtplugPipeClientTransport {
    ButtplugPipeClientTransport {
      address: self.address,
      disconnect_notifier: Arc::new(Notify::new()),
    }
  }
}

async fn run_connection_loop(
  mut client: PipeClientType,
  mut request_receiver: Receiver<ButtplugSerializedMessage>,
  response_sender: Sender<ButtplugTransportIncomingMessage>,
  disconnect_notifier: Arc<Notify>,
) {
  info!("Starting pipe server connection event loop.");

  loop {
    tokio::select! {
      _ = disconnect_notifier.notified()=> {
        info!("Pipe server connector requested disconnect, exiting loop.");
        break;
      },
      serialized_msg = request_receiver.recv() => {
        if let Some(serialized_msg) = serialized_msg {
          match serialized_msg {
            ButtplugSerializedMessage::Text(text_msg) => {
              if client
                .write(text_msg.as_bytes())
                .await
                .is_err() {
                error!("Cannot send text value to server, considering connection closed.");
                break;
              }
            }
            ButtplugSerializedMessage::Binary(binary_msg) => {
              if client
                .write(&binary_msg)
                .await
                .is_err() {
                error!("Cannot send binary value to server, considering connection closed.");
                break;
              }
            }
          }
        } else {
          info!("Pipe server connector owner dropped, disconnecting pipe connection.");
          break;
        }
      }
      ready = client.ready(Interest::READABLE) => {
        match ready {
          Ok(status) => {
            if status.is_readable() {
              let mut data = vec![0; 1024];
              match client.try_read(&mut data) {
                Ok(n) => {
                  if n == 0 {
                    continue;
                  }
                  data.truncate(n);
                  let json_str = if let Ok(json) = String::from_utf8(data) {
                    json
                  } else {
                    error!("Could not parse incoming values as valid utf8.");
                    continue;
                  };
                  if response_sender.send(ButtplugTransportIncomingMessage::Message(ButtplugSerializedMessage::Text(json_str))).await.is_err() {
                    error!("Connector that owns transport no longer available, exiting.");
                    break;
                  }
                },
                Err(err) => {
                  error!("Error from pipe server, assuming disconnection: {:?}", err);
                  break;
                }
              }
            }
          },
          Err(err) => {
            error!("Error from pipe server, assuming disconnection: {:?}", err);
            break;
          }
        }
      }
    }
  }
  let _ = response_sender
    .send(ButtplugTransportIncomingMessage::Close(
      "Pipe server closed".to_owned(),
    ))
    .await;
}

/// Websocket connector for ButtplugClients, using [async_tungstenite]
pub struct ButtplugPipeClientTransport {
  /// Address of the server we'll connect to.
  address: String,
  /// Internally held sender, used for when disconnect is called.
  disconnect_notifier: Arc<Notify>,
}

impl ButtplugConnectorTransport for ButtplugPipeClientTransport {
  fn connect(
    &self,
    outgoing_receiver: Receiver<ButtplugSerializedMessage>,
    incoming_sender: Sender<ButtplugTransportIncomingMessage>,
  ) -> BoxFuture<'static, Result<(), ButtplugConnectorError>> {
    let disconnect_notifier = self.disconnect_notifier.clone();
    let address = self.address.clone();
    Box::pin(async move {
      #[cfg(target = "windows")]
      let client = named_pipe::ClientOptions::new()
        .open(address)
        .map_err(|err| {
          ButtplugConnectorError::TransportSpecificError(
            ButtplugConnectorTransportSpecificError::GenericNetworkError(format!("{}", err)),
          )
        })?;
      #[cfg(not(target = "windows"))]
      let client = UnixStream::connect(address)
        .await
        .map_err(|err| {
          ButtplugConnectorError::TransportSpecificError(
            ButtplugConnectorTransportSpecificError::GenericNetworkError(format!("{}", err)),
          )
        })?;      
      tokio::spawn(async move {
        run_connection_loop(
          client,
          outgoing_receiver,
          incoming_sender,
          disconnect_notifier,
        )
        .await;
      });
      Ok(())
    })
  }

  fn disconnect(self) -> ButtplugConnectorResultFuture {
    let disconnect_notifier = self.disconnect_notifier;
    Box::pin(async move {
      // If we can't send the message, we have no loop, so we're not connected.
      disconnect_notifier.notify_waiters();
      Ok(())
    })
  }
}

#[cfg(test)]
mod test {
  use super::ButtplugPipeClientTransportBuilder;
  use crate::{
    client::ButtplugClient,
    connector::{transport::ButtplugConnectorTransport, ButtplugRemoteClientConnector},
    core::messages::serializer::ButtplugClientJSONSerializer,
    util::async_manager,
  };
  use tokio::sync::mpsc;

  #[test]
  pub fn test_client_transport_error_invalid_pipe() {
    async_manager::block_on(async move {
      let transport = ButtplugPipeClientTransportBuilder::new("notapipe").finish();
      let (_, receiver) = mpsc::channel(1);
      let (sender, _) = mpsc::channel(1);
      assert!(transport.connect(receiver, sender).await.is_err());
    });
  }

  #[test]
  pub fn test_client_error_invalid_pipe() {
    async_manager::block_on(async move {
      let transport = ButtplugPipeClientTransportBuilder::new("notapipe").finish();
      let client = ButtplugClient::new("Test Client");
      assert!(client
        .connect(ButtplugRemoteClientConnector::<
          _,
          ButtplugClientJSONSerializer,
        >::new(transport))
        .await
        .is_err());
    });
  }
}
