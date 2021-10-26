use super::{ButtplugServer, ButtplugServerBuilder, DeviceManager};
use crate::{
  connector::ButtplugConnector,
  core::{
    errors::ButtplugError,
    messages::{
      self, ButtplugClientMessage, ButtplugMessage, ButtplugMessageValidator, ButtplugServerMessage,
    },
  },
  util::{async_manager, stream::convert_broadcast_receiver_to_stream},
};
use futures::{future::Future, select, FutureExt, Stream, StreamExt};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, mpsc, Notify};

// Clone derived here to satisfy tokio broadcast requirements.
#[derive(Clone, Debug)]
pub enum ButtplugRemoteServerEvent {
  Connected(String),
  DeviceAdded(u32, String),
  DeviceRemoved(u32),
  Disconnected,
}

#[derive(Error, Debug)]
pub enum ButtplugServerConnectorError {
  #[error("Cannot bring up server for connection: {0}")]
  ConnectorError(String),
}

pub enum ButtplugServerCommand {
  Disconnect,
}

pub struct ButtplugRemoteServer {
  server: Arc<ButtplugServer>,
  event_sender: broadcast::Sender<ButtplugRemoteServerEvent>,
  disconnect_notifier: Arc<Notify>,
}

async fn run_server<ConnectorType>(
  server: Arc<ButtplugServer>,
  remote_event_sender: broadcast::Sender<ButtplugRemoteServerEvent>,
  connector: ConnectorType,
  mut connector_receiver: mpsc::Receiver<ButtplugClientMessage>,
  disconnect_notifier: Arc<Notify>,
) where
  ConnectorType: ButtplugConnector<ButtplugServerMessage, ButtplugClientMessage> + 'static,
{
  info!("Starting remote server loop");
  let shared_connector = Arc::new(connector);
  let server_receiver = server.event_stream();
  pin_mut!(server_receiver);
  loop {
    select! {
      connector_msg = connector_receiver.recv().fuse() => match connector_msg {
        None => {
          info!("Connector disconnected, exiting loop.");
          break;
        }
        Some(client_message) => {
          trace!("Got message from connector: {:?}", client_message);
          let server_clone = server.clone();
          let connector_clone = shared_connector.clone();
          let remote_event_sender_clone = remote_event_sender.clone();
          async_manager::spawn(async move {
            if let Err(e) = client_message.is_valid() {
              error!("Message not valid: {:?} - Error: {}", client_message, e);
              let mut err_msg = messages::Error::from(ButtplugError::from(e));
              err_msg.set_id(client_message.id());
              connector_clone.send(err_msg.into());
              return;
            }
            match server_clone.parse_message(client_message.clone()).await {
              Ok(ret_msg) => {
                if let ButtplugClientMessage::RequestServerInfo(rsi) = client_message {
                  if remote_event_sender_clone.receiver_count() > 0 {
                    if remote_event_sender_clone.send(ButtplugRemoteServerEvent::Connected(rsi.client_name().clone())).is_err() {
                      error!("Cannot send event to owner, dropping and assuming local server thread has exited.");
                    }
                  }
                }
                if connector_clone.send(ret_msg).await.is_err() {
                  error!("Cannot send reply to server, dropping and assuming remote server thread has exited.");
                }
              },
              Err(err_msg) => {
                if connector_clone.send(err_msg.into()).await.is_err() {
                  error!("Cannot send reply to server, dropping and assuming remote server thread has exited.");
                }
              }
            }
          });
        }
      },
      _ = disconnect_notifier.notified().fuse() => {
        info!("Server disconnected via controller disappearance, exiting loop.");
        break;
      },
      server_msg = server_receiver.next().fuse() => match server_msg {
        None => {
          info!("Server disconnected via server disappearance, exiting loop.");
          break;
        }
        Some(msg) => {
          if remote_event_sender.receiver_count() > 0 {
            match &msg {
              ButtplugServerMessage::DeviceAdded(da) => {
                if remote_event_sender.send(ButtplugRemoteServerEvent::DeviceAdded(da.device_index(), da.device_name().clone())).is_err() {
                  error!("Cannot send event to owner, dropping and assuming local server thread has exited.");
                }
              },
              ButtplugServerMessage::DeviceRemoved(dr) => {
               if remote_event_sender.send(ButtplugRemoteServerEvent::DeviceRemoved(dr.device_index())).is_err() {
                 error!("Cannot send event to owner, dropping and assuming local server thread has exited.");
               }
              },
              _ => {}
            }  
          }
          let connector_clone = shared_connector.clone();
          if connector_clone.send(msg).await.is_err() {
            error!("Server disappeared, exiting remote server thread.");
          }
        }
      },
    };
  }
  if let Err(err) = server.disconnect().await {
    error!("Error disconnecting server: {:?}", err);
  }
  info!("Exiting remote server loop");
}

impl Default for ButtplugRemoteServer {
  fn default() -> Self {
    Self::new(ButtplugServerBuilder::default().finish().expect("Default is infallible"))
  }
}

impl ButtplugRemoteServer {
  pub fn new(server: ButtplugServer) -> Self {
    let (event_sender, _) = broadcast::channel(256);
    Self {
      event_sender,
      server: Arc::new(server),
      disconnect_notifier: Arc::new(Notify::new()),
    }
  }

  pub fn event_stream(&self) -> impl Stream<Item = ButtplugRemoteServerEvent> {
    convert_broadcast_receiver_to_stream(self.event_sender.subscribe())
  }

  pub fn start<ConnectorType>(
    &self,
    mut connector: ConnectorType,
  ) -> impl Future<Output = Result<(), ButtplugServerConnectorError>>
  where
    ConnectorType: ButtplugConnector<ButtplugServerMessage, ButtplugClientMessage> + 'static,
  {
    let server_clone = self.server.clone();
    let event_sender_clone = self.event_sender.clone();
    let disconnect_notifier = self.disconnect_notifier.clone();
    async move {
      let (connector_sender, connector_receiver) = mpsc::channel(256);
      connector
        .connect(connector_sender)
        .await
        .map_err(|e| ButtplugServerConnectorError::ConnectorError(format!("{:?}", e)))?;
      run_server(
        server_clone,
        event_sender_clone,
        connector,
        connector_receiver,
        disconnect_notifier,
      )
      .await;
      Ok(())
    }
  }

  pub async fn disconnect(&self) -> Result<(), ButtplugError> {
    self.disconnect_notifier.notify_waiters();
    Ok(())
  }

  pub fn device_manager(&self) -> &DeviceManager {
    self.server.device_manager()
  }
}

impl Drop for ButtplugRemoteServer {
  fn drop(&mut self) {
    self.disconnect_notifier.notify_waiters();
  }
}
