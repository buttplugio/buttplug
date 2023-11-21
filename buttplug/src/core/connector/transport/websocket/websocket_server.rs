// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    connector::{
      transport::{
        ButtplugConnectorTransport,
        ButtplugConnectorTransportSpecificError,
        ButtplugTransportIncomingMessage,
      },
      ButtplugConnectorError,
      ButtplugConnectorResultFuture,
    },
    message::serializer::ButtplugSerializedMessage,
  },
  util::async_manager,
};
use futures::{future::BoxFuture, FutureExt, SinkExt, StreamExt};
use std::{sync::Arc, time::Duration};
use tokio::{
  net::{TcpListener, TcpStream},
  sync::{
    mpsc::{Receiver, Sender},
    Notify,
  },
  time::sleep,
};

#[derive(Clone, Debug)]
pub struct ButtplugWebsocketServerTransportBuilder {
  /// If true, listens all on available interfaces. Otherwise, only listens on 127.0.0.1.
  listen_on_all_interfaces: bool,
  /// Insecure port for listening for websocket connections.
  port: u16,
}

impl Default for ButtplugWebsocketServerTransportBuilder {
  fn default() -> Self {
    Self {
      listen_on_all_interfaces: false,
      port: 12345,
    }
  }
}

impl ButtplugWebsocketServerTransportBuilder {
  pub fn listen_on_all_interfaces(&mut self, listen_on_all_interfaces: bool) -> &mut Self {
    self.listen_on_all_interfaces = listen_on_all_interfaces;
    self
  }

  pub fn port(&mut self, port: u16) -> &mut Self {
    self.port = port;
    self
  }

  pub fn finish(&self) -> ButtplugWebsocketServerTransport {
    ButtplugWebsocketServerTransport {
      port: self.port,
      listen_on_all_interfaces: self.listen_on_all_interfaces,
      disconnect_notifier: Arc::new(Notify::new()),
    }
  }
}

async fn run_connection_loop(
  ws_stream: tokio_tungstenite::WebSocketStream<TcpStream>,
  mut request_receiver: Receiver<ButtplugSerializedMessage>,
  response_sender: Sender<ButtplugTransportIncomingMessage>,
  disconnect_notifier: Arc<Notify>,
) {
  info!("Starting websocket server connection event loop.");

  let (mut websocket_server_sender, mut websocket_server_receiver) = ws_stream.split();

  // Start pong count at 1, so we'll clear it after sending our first ping.
  let mut pong_count = 1u32;
  loop {
    select! {
      _ = disconnect_notifier.notified().fuse() => {
        info!("Websocket server connector requested disconnect.");
        if websocket_server_sender.close().await.is_err() {
          warn!("Cannot close, assuming connection already closed");
          return;
        }
      },
      _ = sleep(Duration::from_millis(10000)).fuse() => {
        if pong_count == 0 {
          warn!("No pongs received, considering connection closed.");
          return;
        }
        pong_count = 0;
        if websocket_server_sender
          .send(tokio_tungstenite::tungstenite::Message::Ping(vec!(0)))
          .await
          .is_err() {
          warn!("Cannot send ping to client, considering connection closed.");
          return;
        }
      },
      serialized_msg = request_receiver.recv().fuse() => {
        if let Some(serialized_msg) = serialized_msg {
          match serialized_msg {
            ButtplugSerializedMessage::Text(text_msg) => {
              trace!("Sending text message: {}", text_msg);
              if websocket_server_sender
                .send(tokio_tungstenite::tungstenite::Message::Text(text_msg))
                .await
                .is_err() {
                warn!("Cannot send text value to server, considering connection closed.");
                return;
              }
            }
            ButtplugSerializedMessage::Binary(binary_msg) => {
              if websocket_server_sender
                .send(tokio_tungstenite::tungstenite::Message::Binary(binary_msg))
                .await
                .is_err() {
                warn!("Cannot send binary value to server, considering connection closed.");
                return;
              }
            }
          }
        } else {
          info!("Websocket server connector owner dropped, disconnecting websocket connection.");
          if websocket_server_sender.close().await.is_err() {
            warn!("Cannot close, assuming connection already closed");
          }
          return;
        }
      }
      websocket_server_msg = websocket_server_receiver.next().fuse() => match websocket_server_msg {
        Some(ws_data) => {
          match ws_data {
            Ok(msg) => {
              match msg {
                tokio_tungstenite::tungstenite::Message::Text(text_msg) => {
                  trace!("Got text: {}", text_msg);
                  if response_sender.send(ButtplugTransportIncomingMessage::Message(ButtplugSerializedMessage::Text(text_msg))).await.is_err() {
                    warn!("Connector that owns transport no longer available, exiting.");
                    break;
                  }
                }
                tokio_tungstenite::tungstenite::Message::Close(_) => {
                  let _ = response_sender.send(ButtplugTransportIncomingMessage::Close("Websocket server closed".to_owned())).await;
                  // If closing errors out, log it but there's not a lot we can do.
                  if let Err(e) = websocket_server_sender.close().await {
                    error!("{:?}", e);
                  }
                  break;
                }
                tokio_tungstenite::tungstenite::Message::Ping(val) => {
                  if websocket_server_sender
                    .send(tokio_tungstenite::tungstenite::Message::Pong(val))
                    .await
                    .is_err() {
                    warn!("Cannot send pong to client, considering connection closed.");
                    return;
                  }
                  continue;
                }
                tokio_tungstenite::tungstenite::Message::Frame(_) => {
                  // noop
                  continue;
                }
                tokio_tungstenite::tungstenite::Message::Pong(_) => {
                  pong_count += 1;
                  continue;
                }
                tokio_tungstenite::tungstenite::Message::Binary(_) => {
                  error!("Don't know how to handle binary message types!");
                }
              }
            },
            Err(err) => {
              warn!("Error from websocket server, assuming disconnection: {:?}", err);
              let _ = response_sender.send(ButtplugTransportIncomingMessage::Close("Websocket server closed".to_owned())).await;
              break;
            }
          }
        },
        None => {
          warn!("Websocket channel closed, breaking");
          return;
        }
      }
    }
  }
}

/// Websocket connector for ButtplugClients, using [tokio_tungstenite]
pub struct ButtplugWebsocketServerTransport {
  port: u16,
  listen_on_all_interfaces: bool,
  disconnect_notifier: Arc<Notify>,
}

impl ButtplugConnectorTransport for ButtplugWebsocketServerTransport {
  fn connect(
    &self,
    outgoing_receiver: Receiver<ButtplugSerializedMessage>,
    incoming_sender: Sender<ButtplugTransportIncomingMessage>,
  ) -> BoxFuture<'static, Result<(), ButtplugConnectorError>> {
    let disconnect_notifier = self.disconnect_notifier.clone();

    let base_addr = if self.listen_on_all_interfaces {
      "0.0.0.0"
    } else {
      "127.0.0.1"
    };

    let addr = format!("{}:{}", base_addr, self.port);
    debug!("Websocket: Trying to listen on {}", addr);
    let response_sender_clone = incoming_sender;
    let disconnect_notifier_clone = disconnect_notifier;
    let fut = async move {
      // Create the event loop and TCP listener we'll accept connections on.
      let try_socket = TcpListener::bind(&addr).await;
      debug!("Websocket: Socket bound.");
      let listener = try_socket.map_err(|e| {
        ButtplugConnectorError::TransportSpecificError(
          ButtplugConnectorTransportSpecificError::GenericNetworkError(format!("{:?}", e)),
        )
      })?;
      debug!("Websocket: Listening on: {}", addr);
      if let Ok((stream, _)) = listener.accept().await {
        info!("Websocket: Got connection");
        let ws_stream = tokio_tungstenite::accept_async(stream)
          .await
          .map_err(|err| {
            error!("Websocket server accept error: {:?}", err);
            ButtplugConnectorError::TransportSpecificError(
              ButtplugConnectorTransportSpecificError::TungsteniteError(err),
            )
          })?;
        async_manager::spawn(async move {
          run_connection_loop(
            ws_stream,
            outgoing_receiver,
            response_sender_clone,
            disconnect_notifier_clone,
          )
          .await;
        });
        Ok(())
      } else {
        Err(ButtplugConnectorError::ConnectorGenericError(
          "Could not run accept for port".to_owned(),
        ))
      }
    };

    async move { fut.await }.boxed()
  }

  fn disconnect(self) -> ButtplugConnectorResultFuture {
    let disconnect_notifier = self.disconnect_notifier;
    async move {
      disconnect_notifier.notify_waiters();
      Ok(())
    }
    .boxed()
  }
}
