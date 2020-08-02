use super::ButtplugServer;
use crate::{
  core::{
    errors::{ButtplugError, ButtplugServerError},
    messages::{ButtplugClientMessage, ButtplugServerMessage},
  },
  connector::ButtplugConnector,
  util::async_manager,
  server::comm_managers::btleplug::BtlePlugCommunicationManager
};
use async_channel::{Sender, Receiver, bounded};
use async_mutex::Mutex;
use std::sync::Arc;
use thiserror::Error;
use futures::{StreamExt, future::Future, select, FutureExt};

pub enum ButtplugServerEvent {
  Connected(String),
  DeviceAdded(String),
  DeviceRemoved(String),
  Disconnected,
}

#[derive(Error, Debug)]
pub enum ButtplugServerConnectorError {
  #[error("Can't connect")]
  ConnectorError
}

pub enum ButtplugServerCommand {
  Disconnect
}

pub struct ButtplugRemoteServer {
  server: Arc<ButtplugServer>,
  server_receiver: Receiver<ButtplugServerMessage>,
  task_channel: Arc<Mutex<Option<Sender<ButtplugServerCommand>>>>,
}

async fn run_server<ConnectorType>(
  server: Arc<ButtplugServer>,
  mut server_receiver: Receiver<ButtplugServerMessage>,
  connector: ConnectorType,
  mut connector_receiver: Receiver<Result<ButtplugClientMessage, ButtplugServerError>>,
  mut controller_receiver: Receiver<ButtplugServerCommand>
) where ConnectorType: ButtplugConnector<ButtplugServerMessage, ButtplugClientMessage> + 'static {
  info!("Starting remote server loop");
  let shared_connector = Arc::new(connector);
  loop {
    select! {
      connector_msg = connector_receiver.next().fuse() => match connector_msg {
        None => {
          info!("Connector disconnected, exiting loop.");
          break;
        }
        Some(msg) => {
          info!("Got message from connector: {:?}", msg);
          let server_clone = server.clone();
          let connector_clone = shared_connector.clone();
          async_manager::spawn(async move {
            // TODO This isn't handling server errors correctly
            let ret_msg = server_clone.parse_message(msg.unwrap()).await.unwrap();
            connector_clone.send(ret_msg).await;
          });
        }
      },
      controller_msg = controller_receiver.next().fuse() => match controller_msg {
        None => {
          info!("Server disconnected via controller request, exiting loop.");
          break;
        }
        Some(msg) => { 
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
          let connector_clone = shared_connector.clone();
          async_manager::spawn(async move {
            connector_clone.send(msg).await;
          });
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
  pub fn new(name: &str, max_ping_time: u64) -> Self {
    let (mut server, server_receiver) = ButtplugServer::new(name, max_ping_time);
    server.add_comm_manager::<BtlePlugCommunicationManager>();
    Self {
      server: Arc::new(server),
      server_receiver,
      task_channel: Arc::new(Mutex::new(None)),
    }
  }

  pub fn start<ConnectorType>(
    &self,
    mut connector: ConnectorType,
  ) -> impl Future<Output = Result<(), ButtplugServerConnectorError>>
  where
  ConnectorType: ButtplugConnector<ButtplugServerMessage, ButtplugClientMessage> + 'static {
    let task_channel = self.task_channel.clone();
    let server_clone = self.server.clone();
    let server_receiver_clone = self.server_receiver.clone();
    async move {
      let connector_receiver= connector.connect().await.map_err(|_| ButtplugServerConnectorError::ConnectorError)?;
      let (controller_sender, controller_receiver) = bounded(256);
      let mut locked_channel = task_channel.lock().await;
      *locked_channel = Some(controller_sender);
      run_server(server_clone, server_receiver_clone, connector, connector_receiver, controller_receiver).await;
      Ok(())
    }
  }

  pub fn disconnect(&self) -> impl Future<Output=Result<(), ButtplugError>> {
    async move {
      Ok(())
    }
  }
}