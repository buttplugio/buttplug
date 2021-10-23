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
      ButtplugTransportIncomingMessage,
    },
    ButtplugConnectorError, ButtplugConnectorResultFuture,
  },
  core::messages::serializer::ButtplugSerializedMessage,
};
use futures::future::BoxFuture;
use std::sync::Arc;
use tokio::{
  sync::{
    mpsc::{Receiver, Sender},
    Notify,
  },
  io::{AsyncWriteExt, Interest}
};
#[cfg(target_os = "windows")]
use tokio::net::windows::named_pipe;
use tracing::Instrument;

#[derive(Clone, Debug)]
pub struct ButtplugPipeClientTransportBuilder {
  /// Address (either Named Pipe or Domain Socket File) to connect to
  address: String,
}

impl ButtplugPipeClientTransportBuilder {
  pub fn new(address: &str) -> Self {
    Self {
      address: address.to_owned()
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
  pipe_name: &str,
  mut request_receiver: Receiver<ButtplugSerializedMessage>,
  response_sender: Sender<ButtplugTransportIncomingMessage>,
  disconnect_notifier: Arc<Notify>,
) {
  info!("Starting pipe server connection event loop.");

  let mut client = named_pipe::ClientOptions::new()
    .open(pipe_name)
    .unwrap();

  loop {
    tokio::select! {
      _ = disconnect_notifier.notified()=> {
        info!("Pipe server connector requested disconnect.");
        /*
        if client.disconnect().is_err() {
          error!("Cannot close, assuming connection already closed");
          return;
        }
        */
        return;
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
                return;
              }
            }
            ButtplugSerializedMessage::Binary(binary_msg) => {
              if client
                .write(&binary_msg)
                .await
                .is_err() {
                error!("Cannot send binary value to server, considering connection closed.");
                return;
              }
            }
          }
        } else {
          info!("Websocket server connector owner dropped, disconnecting websocket connection.");
          /*
          if client.disconnect().is_err() {
            error!("Cannot close, assuming connection already closed");
          }
           */
          return;
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
                  if response_sender.send(ButtplugTransportIncomingMessage::Message(ButtplugSerializedMessage::Text(String::from_utf8(data).unwrap()))).await.is_err() {
                    error!("Connector that owns transport no longer available, exiting.");
                    break;
                  }
    
                },
                Err(e) => {

                }
              }
            }
          },
          Err(err) => {
            error!("Error from websocket server, assuming disconnection: {:?}", err);
            let _ = response_sender.send(ButtplugTransportIncomingMessage::Close("Websocket server closed".to_owned())).await;
            break;
          }
        }
      }
    }
  }
}

/// Websocket connector for ButtplugClients, using [async_tungstenite]
pub struct ButtplugPipeClientTransport {
  /// Address of the server we'll connect to.
  address: String,
  /// Internally held sender, used for when disconnect is called.
  disconnect_notifier: Arc<Notify>,
}

impl ButtplugPipeClientTransport {
  fn create(address: &str) -> Self {
    Self {
      address: address.to_owned(),
      disconnect_notifier: Arc::new(Notify::new()),
    }
  }
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
      tokio::spawn(async move {
        run_connection_loop(&address, outgoing_receiver, incoming_sender, disconnect_notifier).await;
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
