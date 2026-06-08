// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::{
  connector::{
    ButtplugConnectorError,
    ButtplugConnectorResultFuture,
    transport::{
      ButtplugConnectorTransport,
      ButtplugConnectorTransportSpecificError,
      ButtplugTransportIncomingMessage,
    },
  },
  message::serializer::ButtplugSerializedMessage,
};
use futures::{FutureExt, SinkExt, StreamExt, future::BoxFuture};
use std::{fmt, sync::Arc, time::Duration};
use tokio::{
  net::{TcpListener, TcpStream},
  select,
  sync::{
    Notify,
    mpsc::{Receiver, Sender},
  },
  time::sleep,
};

#[derive(Clone)]
struct ListenerBoundCallback(Arc<dyn Fn(u16) + Send + Sync>);

impl ListenerBoundCallback {
  fn new(callback: impl Fn(u16) + Send + Sync + 'static) -> Self {
    Self(Arc::new(callback))
  }

  fn call(&self, port: u16) {
    (self.0)(port);
  }
}

impl fmt::Debug for ListenerBoundCallback {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ListenerBoundCallback")
      .finish_non_exhaustive()
  }
}

#[derive(Clone, Debug)]
pub struct ButtplugWebsocketServerTransportBuilder {
  /// If true, listens all on available interfaces. Otherwise, only listens on 127.0.0.1.
  listen_on_all_interfaces: bool,
  /// Insecure port for listening for websocket connections.
  port: u16,
  /// Optional callback fired after the listener is bound and the actual local port is known.
  listener_bound_callback: Option<ListenerBoundCallback>,
}

impl Default for ButtplugWebsocketServerTransportBuilder {
  fn default() -> Self {
    Self {
      listen_on_all_interfaces: false,
      port: 12345,
      listener_bound_callback: None,
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

  pub fn on_listener_bound(&mut self, callback: impl Fn(u16) + Send + Sync + 'static) -> &mut Self {
    self.listener_bound_callback = Some(ListenerBoundCallback::new(callback));
    self
  }

  pub fn finish(&self) -> ButtplugWebsocketServerTransport {
    ButtplugWebsocketServerTransport {
      port: self.port,
      listen_on_all_interfaces: self.listen_on_all_interfaces,
      listener_bound_callback: self.listener_bound_callback.clone(),
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
          .send(tokio_tungstenite::tungstenite::Message::Ping(vec!(0).into()))
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
                .send(tokio_tungstenite::tungstenite::Message::Text(text_msg.into()))
                .await
                .is_err() {
                warn!("Cannot send text value to server, considering connection closed.");
                return;
              }
            }
            ButtplugSerializedMessage::Binary(binary_msg) => {
              if websocket_server_sender
                .send(tokio_tungstenite::tungstenite::Message::Binary(binary_msg.into()))
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
                  if response_sender.send(ButtplugTransportIncomingMessage::Message(ButtplugSerializedMessage::Text(text_msg.as_str().to_owned()))).await.is_err() {
                    warn!("Connector that owns transport no longer available, exiting.");
                    break;
                  }
                }
                tokio_tungstenite::tungstenite::Message::Close(_) => {
                  let _ = response_sender.send(ButtplugTransportIncomingMessage::Close("Websocket server closed".to_owned())).await;
                  // If closing errors out, log it but there's not a lot we can do.
                  if let Err(e) = websocket_server_sender.close().await {
                    error!("Error closing websocket: {:?}", e);
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
  listener_bound_callback: Option<ListenerBoundCallback>,
  disconnect_notifier: Arc<Notify>,
}

impl ButtplugConnectorTransport for ButtplugWebsocketServerTransport {
  fn connect(
    &self,
    outgoing_receiver: Receiver<ButtplugSerializedMessage>,
    incoming_sender: Sender<ButtplugTransportIncomingMessage>,
  ) -> BoxFuture<'static, Result<(), ButtplugConnectorError>> {
    let disconnect_notifier = self.disconnect_notifier.clone();
    let listener_bound_callback = self.listener_bound_callback.clone();

    let base_addr = if self.listen_on_all_interfaces {
      "0.0.0.0"
    } else {
      "127.0.0.1"
    };

    let address = base_addr.to_owned();
    let port = self.port;
    let addr = format!("{}:{}", address, port);
    debug!("Websocket: Trying to listen on {}", addr);
    let response_sender_clone = incoming_sender;
    let disconnect_notifier_clone = disconnect_notifier;
    let fut = async move {
      // Create the event loop and TCP listener we'll accept connections on.
      let try_socket = TcpListener::bind(&addr).await;
      debug!("Websocket: Socket bound.");
      let listener = try_socket.map_err(|e| {
        ButtplugConnectorError::TransportSpecificError(
          ButtplugConnectorTransportSpecificError::SocketBindError {
            address,
            port,
            kind: e.kind(),
            message: e.to_string(),
          },
        )
      })?;
      debug!("Websocket: Listening on: {}", addr);
      if let Some(callback) = &listener_bound_callback {
        let local_port = listener
          .local_addr()
          .map_err(|e| {
            ButtplugConnectorError::TransportSpecificError(
              ButtplugConnectorTransportSpecificError::GenericNetworkError(format!(
                "Could not determine websocket listener local address: {e}"
              )),
            )
          })?
          .port();
        callback.call(local_port);
      }
      if let Ok((stream, _)) = listener.accept().await {
        info!("Websocket: Got connection");
        let ws_stream = tokio_tungstenite::accept_async(stream)
          .await
          .map_err(|err| {
            error!("Websocket server accept error: {:?}", err);
            ButtplugConnectorError::TransportSpecificError(
              ButtplugConnectorTransportSpecificError::GenericNetworkError(format!("{err:?}")),
            )
          })?;
        buttplug_core::spawn!(
          "ButtplugWebsocketServerTransport connection loop",
          async move {
            run_connection_loop(
              ws_stream,
              outgoing_receiver,
              response_sender_clone,
              disconnect_notifier_clone,
            )
            .await;
          }
        );
        Ok(())
      } else {
        Err(ButtplugConnectorError::ConnectorGenericError(
          "Could not run accept for port".to_owned(),
        ))
      }
    };

    fut.boxed()
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

#[cfg(test)]
mod test {
  use super::ButtplugWebsocketServerTransportBuilder;
  use buttplug_core::{
    connector::{
      ButtplugConnectorError,
      transport::{
        ButtplugConnectorTransport,
        ButtplugConnectorTransportSpecificError,
        ButtplugTransportIncomingMessage,
      },
    },
    message::serializer::ButtplugSerializedMessage,
  };
  use std::io::ErrorKind;
  use std::sync::{Arc, Mutex};
  use tokio::{net::TcpListener, sync::mpsc};

  #[tokio::test]
  async fn bind_addr_in_use_returns_structured_error() {
    let _listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = _listener.local_addr().unwrap().port();
    let transport = ButtplugWebsocketServerTransportBuilder::default()
      .port(port)
      .finish();
    let (_outgoing_sender, outgoing_receiver) = mpsc::channel::<ButtplugSerializedMessage>(1);
    let (incoming_sender, _incoming_receiver) =
      mpsc::channel::<ButtplugTransportIncomingMessage>(1);

    let err = transport
      .connect(outgoing_receiver, incoming_sender)
      .await
      .unwrap_err();

    match err {
      ButtplugConnectorError::TransportSpecificError(
        ButtplugConnectorTransportSpecificError::SocketBindError {
          address,
          port: error_port,
          kind,
          message: _,
        },
      ) => {
        assert_eq!(address, "127.0.0.1");
        assert_eq!(error_port, port);
        assert_eq!(kind, ErrorKind::AddrInUse);
      }
      other => panic!("Unexpected error: {other:?}"),
    }
  }

  #[tokio::test]
  async fn listener_bound_callback_receives_actual_port() {
    let bound_port = Arc::new(Mutex::new(None));
    let callback_port = bound_port.clone();
    let transport = ButtplugWebsocketServerTransportBuilder::default()
      .on_listener_bound(move |port| {
        *callback_port.lock().unwrap() = Some(port);
      })
      .finish();
    let (_outgoing_sender, outgoing_receiver) = mpsc::channel::<ButtplugSerializedMessage>(1);
    let (incoming_sender, _incoming_receiver) =
      mpsc::channel::<ButtplugTransportIncomingMessage>(1);
    let connect_task = tokio::spawn(async move {
      let _ = transport.connect(outgoing_receiver, incoming_sender).await;
    });

    tokio::time::timeout(std::time::Duration::from_secs(1), async {
      loop {
        if let Some(port) = *bound_port.lock().unwrap() {
          return port;
        }
        tokio::task::yield_now().await;
      }
    })
    .await
    .expect("listener bound callback was not called");

    let port = bound_port.lock().unwrap().unwrap();
    assert!(port > 0);

    connect_task.abort();
  }
}
