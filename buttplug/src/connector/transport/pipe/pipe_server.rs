use crate::{
  connector::{
    transport::{
      ButtplugConnectorTransport,
      ButtplugConnectorTransportSpecificError,
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
  mut server: named_pipe::NamedPipeServer,
  mut request_receiver: Receiver<ButtplugSerializedMessage>,
  response_sender: Sender<ButtplugTransportIncomingMessage>,
  disconnect_notifier: Arc<Notify>,
) {
  info!("Starting pipe server connection event loop.");

  loop {
    tokio::select! {
      _ = disconnect_notifier.notified()=> {
        info!("Pipe server connector requested disconnect.");
        if server.disconnect().is_err() {
          error!("Cannot close, assuming connection already closed");
          break;
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
                break;
              }
            }
            ButtplugSerializedMessage::Binary(binary_msg) => {
              if server
                .write(&binary_msg)
                .await
                .is_err() {
                error!("Cannot send binary value to server, considering connection closed.");
                break;
              }
            }
          }
        } else {
          info!("Pipe server connector owner dropped, disconnecting websocket connection.");
          if server.disconnect().is_err() {
            error!("Cannot close, assuming connection already closed");
          }
          break;
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
  let _ = response_sender.send(ButtplugTransportIncomingMessage::Close("Pipe server closed".to_owned())).await;
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
      let server = named_pipe::ServerOptions::new()
        .first_pipe_instance(true)
        .create(address)
        .map_err(|err| ButtplugConnectorError::TransportSpecificError(ButtplugConnectorTransportSpecificError::GenericNetworkError(format!("{}", err))))?;
      server.connect().await.map_err(|err| ButtplugConnectorError::TransportSpecificError(ButtplugConnectorTransportSpecificError::GenericNetworkError(format!("{}", err))))?;
  
      tokio::spawn(async move {
        run_connection_loop(server, outgoing_receiver, incoming_sender, disconnect_notifier).await;
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



#[cfg(test)]
mod test {
  use super::ButtplugPipeServerTransportBuilder;
  use crate::{
    core::messages::serializer::ButtplugServerJSONSerializer,
    connector::{ButtplugRemoteServerConnector, transport::ButtplugConnectorTransport},
    util::async_manager,
    server::ButtplugRemoteServer
  };
  use tokio::sync::mpsc;


  #[test]
  pub fn test_server_transport_error_invalid_pipe() {
    async_manager::block_on(async move {
      let transport = ButtplugPipeServerTransportBuilder::new("notapipe").finish();
      let (_, receiver) = mpsc::channel(1);
      let (sender, _) = mpsc::channel(1);
      assert!(transport.connect(receiver, sender).await.is_err());
    });
  }

  #[test]
  pub fn test_server_error_invalid_pipe() {
    async_manager::block_on(async move {
      let transport = ButtplugPipeServerTransportBuilder::new("notapipe").finish();
      let server = ButtplugRemoteServer::default();
      assert!(server.start(ButtplugRemoteServerConnector::<_, ButtplugServerJSONSerializer>::new(transport)).await.is_err());
    });
  }
}