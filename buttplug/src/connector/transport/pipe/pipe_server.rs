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


#[derive(Clone, Debug)]
pub struct ButtplugPipeServerTransportBuilder {
  /// Address (either Named Pipe or Domain Socket File) to connect to
  address: String,
}

impl ButtplugPipeServerTransportBuilder {
  pub fn new(address: &str) -> Self {
    Self {
      address: address.to_owned()
    }
  }

  pub fn finish(self) -> ButtplugPipeServerTransport {
    ButtplugPipeServerTransport {
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

  let mut server = named_pipe::ServerOptions::new()
    .first_pipe_instance(true)
    .create(pipe_name)
    .unwrap();
  server.connect().await.unwrap();

  loop {
    tokio::select! {
      _ = disconnect_notifier.notified()=> {
        info!("Pipe server connector requested disconnect.");
        if server.disconnect().is_err() {
          error!("Cannot close, assuming connection already closed");
          return;
        }
      },
      serialized_msg = request_receiver.recv() => {
        if let Some(serialized_msg) = serialized_msg {
          match serialized_msg {
            ButtplugSerializedMessage::Text(text_msg) => {
              if server
                .write(text_msg.as_bytes())
                .await
                .is_err() {
                error!("Cannot send text value to server, considering connection closed.");
                return;
              }
            }
            ButtplugSerializedMessage::Binary(binary_msg) => {
              if server
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
          if server.disconnect().is_err() {
            error!("Cannot close, assuming connection already closed");
          }
          return;
        }
      }
      ready = server.ready(Interest::READABLE) => {
        match ready {
          Ok(status) => {
            if status.is_readable() {
              let mut data = vec![0; 1024];
              match server.try_read(&mut data) {
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
pub struct ButtplugPipeServerTransport {
  address: String,
  disconnect_notifier: Arc<Notify>,
}

impl ButtplugConnectorTransport for ButtplugPipeServerTransport {
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
      disconnect_notifier.notify_waiters();
      Ok(())
    })
  }
}
