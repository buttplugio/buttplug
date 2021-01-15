use super::{ButtplugServer, ButtplugServerOptions, ButtplugServerStartupError};
use crate::{
  connector::ButtplugConnector,
  core::{
    errors::{ButtplugError, ButtplugServerError},
    messages::{self, ButtplugClientMessage, ButtplugServerMessage},
  },
  server::{DeviceCommunicationManager, DeviceCommunicationManagerCreator},
  test::TestDeviceCommunicationManagerHelper,
  util::async_manager,
};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use async_lock::Mutex;
use futures::{future::Future, select, FutureExt, StreamExt, Stream};
use std::sync::Arc;
use thiserror::Error;

pub enum ButtplugRemoteServerEvent {
  Connected(String),
  DeviceAdded(u32, String),
  DeviceRemoved(u32),
  Disconnected,
}

#[derive(Error, Debug)]
pub enum ButtplugServerConnectorError {
  #[error("Can't connect")]
  ConnectorError,
}

pub enum ButtplugServerCommand {
  Disconnect,
}

pub struct ButtplugRemoteServer {
  server: Arc<ButtplugServer>,
  pub(super) event_sender: Sender<ButtplugRemoteServerEvent>,
  task_channel: Arc<Mutex<Option<Sender<ButtplugServerCommand>>>>,
}

async fn run_server<ConnectorType>(
  server: Arc<ButtplugServer>,
  remote_event_sender: Sender<ButtplugRemoteServerEvent>,
  connector: ConnectorType,
  mut connector_receiver: Receiver<Result<ButtplugClientMessage, ButtplugServerError>>,
  mut controller_receiver: Receiver<ButtplugServerCommand>,
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
        Some(msg) => {
          debug!("Got message from connector: {:?}", msg);
          let server_clone = server.clone();
          let connector_clone = shared_connector.clone();
          let remote_event_sender_clone = remote_event_sender.clone();
          async_manager::spawn(async move {
            let client_message = msg.unwrap();
            match server_clone.parse_message(client_message.clone()).await {
              Ok(ret_msg) => {
                if let ButtplugClientMessage::RequestServerInfo(rsi) = client_message {
                  if remote_event_sender_clone.send(ButtplugRemoteServerEvent::Connected(rsi.client_name)).await.is_err() {
                    error!("Cannot send event to owner, dropping and assuming local server thread has exited.");
                  }
                }
                if connector_clone.send(ret_msg).await.is_err() {
                  error!("Cannot send reply to server, dropping and assuming remote server thread has exited.");
                }
              },
              Err(err_msg) => {
                if connector_clone.send(messages::Error::from(err_msg).into()).await.is_err() {
                  error!("Cannot send reply to server, dropping and assuming remote server thread has exited.");
                }
              }
            }
          }).unwrap();
        }
      },
      controller_msg = controller_receiver.recv().fuse() => match controller_msg {
        None => {
          info!("Server disconnected via controller request, exiting loop.");
          break;
        }
        Some(_) => {
          info!("Server disconnected via controller disappearance, exiting loop.");
          break;
        }
      },
      server_msg = server_receiver.next().fuse() => match server_msg {
        None => {
          info!("Server disconnected via server disappearance, exiting loop.");
          break;
        }
        Some(msg) => {
          match &msg {
            ButtplugServerMessage::DeviceAdded(da) => {
              if remote_event_sender.send(ButtplugRemoteServerEvent::DeviceAdded(da.device_index, da.device_name.clone())).await.is_err() {
                error!("Cannot send event to owner, dropping and assuming local server thread has exited.");
              }
            },
            ButtplugServerMessage::DeviceRemoved(dr) => {
             if remote_event_sender.send(ButtplugRemoteServerEvent::DeviceRemoved(dr.device_index)).await.is_err() {
               error!("Cannot send event to owner, dropping and assuming local server thread has exited.");
             }
            },
            _ => {}
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

impl ButtplugRemoteServer {
  // Can't use the Default trait because we need to return our stream, so this
  // is the next best thing.
  pub fn default() -> (Self, Receiver<ButtplugRemoteServerEvent>) {
    Self::new_with_options(&ButtplugServerOptions::default()).unwrap()
  }

  pub fn new_with_options(
    options: &ButtplugServerOptions,
  ) -> Result<(Self, Receiver<ButtplugRemoteServerEvent>), ButtplugError> {
    let server = ButtplugServer::new_with_options(options)?;
    let (remote_event_sender, remote_event_receiver) = channel(256);
    Ok((
      Self {
        event_sender: remote_event_sender,
        server: Arc::new(server),
        task_channel: Arc::new(Mutex::new(None)),
      },
      remote_event_receiver,
    ))
  }

  pub fn event_stream(&self) -> impl Stream<Item = ButtplugServerMessage> {
    self.server.event_stream()
  }

  pub fn start<ConnectorType>(
    &self,
    mut connector: ConnectorType,
  ) -> impl Future<Output = Result<(), ButtplugServerConnectorError>>
  where
    ConnectorType: ButtplugConnector<ButtplugServerMessage, ButtplugClientMessage> + 'static,
  {
    let task_channel = self.task_channel.clone();
    let server_clone = self.server.clone();
    let event_sender_clone = self.event_sender.clone();
    async move {
      let connector_receiver = connector
        .connect()
        .await
        .map_err(|_| ButtplugServerConnectorError::ConnectorError)?;
      let (controller_sender, controller_receiver) = channel(256);
      let mut locked_channel = task_channel.lock().await;
      *locked_channel = Some(controller_sender);
      run_server(
        server_clone,
        event_sender_clone,
        connector,
        connector_receiver,
        controller_receiver,
      )
      .await;
      Ok(())
    }
  }

  pub async fn disconnect(&self) -> Result<(), ButtplugError> {
    Ok(())
  }

  pub fn add_comm_manager<T>(&self) -> Result<(), ButtplugServerStartupError>
  where
    T: 'static + DeviceCommunicationManager + DeviceCommunicationManagerCreator,
  {
    self.server.add_comm_manager::<T>()
  }

  pub fn add_test_comm_manager(
    &self,
  ) -> Result<TestDeviceCommunicationManagerHelper, ButtplugServerStartupError> {
    self.server.add_test_comm_manager()
  }
}
