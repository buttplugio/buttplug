use crate::{
  connector::{
    transport::{
      ButtplugConnectorTransport, ButtplugConnectorTransportSpecificError,
      ButtplugTransportIncomingMessage,
    },
    ButtplugConnectorError, ButtplugConnectorResultFuture,
  },
  core::messages::serializer::ButtplugSerializedMessage,
  util::async_manager,
};
use futures::{future::BoxFuture, AsyncRead, AsyncWrite, FutureExt, SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{
  mpsc::{Receiver, Sender},
  Mutex, Notify,
};

#[derive(Default, Clone, Debug)]
pub struct ButtplugWebsocketServerTransportOptions {
  /// If true, listens all on available interfaces. Otherwise, only listens on 127.0.0.1.
  pub ws_listen_on_all_interfaces: bool,
  /// Insecure port for listening for websocket connections. Secure ports were
  /// removed, but this name was left as is to minimize code breakage.
  pub ws_insecure_port: u16,
}

async fn run_connection_loop<S>(
  ws_stream: async_tungstenite::WebSocketStream<S>,
  mut request_receiver: Receiver<ButtplugSerializedMessage>,
  response_sender: Sender<ButtplugTransportIncomingMessage>,
  disconnect_notifier: Arc<Notify>,
) where
  S: AsyncRead + AsyncWrite + Unpin,
{
  info!("Starting websocket server connection event loop.");

  let (mut websocket_server_sender, mut websocket_server_receiver) = ws_stream.split();

  loop {
    select! {
      _ = disconnect_notifier.notified().fuse() => {
        info!("Websocket server connector requested disconnect.");
        if websocket_server_sender.close().await.is_err() {
          error!("Cannot close, assuming connection already closed");
          return;
        }
      },
      serialized_msg = request_receiver.recv().fuse() => {
        if let Some(serialized_msg) = serialized_msg {
          match serialized_msg {
            ButtplugSerializedMessage::Text(text_msg) => {
              if websocket_server_sender
                .send(async_tungstenite::tungstenite::Message::Text(text_msg))
                .await
                .is_err() {
                error!("Cannot send text value to server, considering connection closed.");
                return;
              }
            }
            ButtplugSerializedMessage::Binary(binary_msg) => {
              if websocket_server_sender
                .send(async_tungstenite::tungstenite::Message::Binary(binary_msg))
                .await
                .is_err() {
                error!("Cannot send binary value to server, considering connection closed.");
                return;
              }
            }
          }
        } else {
          info!("Websocket server connector owner dropped, disconnecting websocket connection.");
          if websocket_server_sender.close().await.is_err() {
            error!("Cannot close, assuming connection already closed");
          }
          return;
        }
      }
      websocket_server_msg = websocket_server_receiver.next().fuse() => match websocket_server_msg {
        Some(ws_data) => {
          match ws_data {
            Ok(msg) => {
              match msg {
                async_tungstenite::tungstenite::Message::Text(text_msg) => {
                  trace!("Got text: {}", text_msg);
                  if response_sender.send(ButtplugTransportIncomingMessage::Message(ButtplugSerializedMessage::Text(text_msg))).await.is_err() {
                    error!("Connector that owns transport no longer available, exiting.");
                    break;
                  }
                }
                async_tungstenite::tungstenite::Message::Close(_) => {
                  let _ = response_sender.send(ButtplugTransportIncomingMessage::Close("Websocket server closed".to_owned())).await;
                  break;
                }
                async_tungstenite::tungstenite::Message::Ping(_) => {
                  // noop
                  continue;
                }
                async_tungstenite::tungstenite::Message::Pong(_) => {
                  // noop
                  continue;
                }
                async_tungstenite::tungstenite::Message::Binary(_) => {
                  error!("Don't know how to handle binary message types!");
                }
              }
            },
            Err(err) => {
              error!("Error from websocket server, assuming disconnection: {:?}", err);
              let _ = response_sender.send(ButtplugTransportIncomingMessage::Close("Websocket server closed".to_owned())).await;
              break;
            }
          }
        },
        None => {
          error!("Websocket channel closed, breaking");
          return;
        }
      }
    }
  }
}

/// Websocket connector for ButtplugClients, using [async_tungstenite]
pub struct ButtplugWebsocketServerTransport {
  options: ButtplugWebsocketServerTransportOptions,
  disconnect_notifier: Arc<Notify>,
}

impl ButtplugWebsocketServerTransport {
  pub fn new(options: ButtplugWebsocketServerTransportOptions) -> Self {
    Self {
      options,
      disconnect_notifier: Arc::new(Notify::new()),
    }
  }
}

impl ButtplugConnectorTransport for ButtplugWebsocketServerTransport {
  fn connect(
    &self,
    outgoing_receiver: Receiver<ButtplugSerializedMessage>,
    incoming_sender: Sender<ButtplugTransportIncomingMessage>,
  ) -> BoxFuture<'static, Result<(), ButtplugConnectorError>> {
    let disconnect_notifier = self.disconnect_notifier.clone();

    let base_addr = if self.options.ws_listen_on_all_interfaces {
      "0.0.0.0"
    } else {
      "127.0.0.1"
    };

    let request_receiver = Arc::new(Mutex::new(Some(outgoing_receiver)));

    let addr = format!("{}:{}", base_addr, self.options.ws_insecure_port);
    debug!("Websocket Insecure: Trying to listen on {}", addr);
    let request_receiver_clone = request_receiver.clone();
    let response_sender_clone = incoming_sender.clone();
    let disconnect_notifier_clone = disconnect_notifier.clone();
    let fut = async move {
      // Create the event loop and TCP listener we'll accept connections on.
      let try_socket = TcpListener::bind(&addr).await;
      debug!("Websocket Insecure: Socket bound.");
      let listener = try_socket.expect("Failed to bind");
      debug!("Websocket Insecure: Listening on: {}", addr);
      if let Ok((stream, _)) = listener.accept().await {
        info!("Websocket Insecure: Got connection");
        let ws_fut = async_tungstenite::tokio::accept_async(stream);
        let ws_stream = ws_fut.await.map_err(|err| {
          error!("Websocket server accept error: {:?}", err);
          ButtplugConnectorError::TransportSpecificError(
            ButtplugConnectorTransportSpecificError::TungsteniteError(err),
          )
        })?;
        async_manager::spawn(async move {
          run_connection_loop(
            ws_stream,
            (*request_receiver_clone.lock().await).take().unwrap(),
            response_sender_clone,
            disconnect_notifier_clone,
          )
          .await;
        })
        .unwrap();
        Ok(())
      } else {
        Err(ButtplugConnectorError::ConnectorGenericError(
          "Could not run accept for insecure port".to_owned(),
        ))
      }
    };

    Box::pin(async move {
      fut.await?;
      Ok(())
    })
  }

  fn disconnect(self) -> ButtplugConnectorResultFuture {
    let disconnect_notifier = self.disconnect_notifier;
    Box::pin(async move {
      disconnect_notifier.notify_waiters();
      Ok(())
    })
  }
}
