// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::{
    connector::ButtplugConnector,
    errors::ButtplugError,
    message::ButtplugServerMessageV4,
      util::{async_manager, stream::convert_broadcast_receiver_to_stream},
};
use buttplug_server_device_config::UserDeviceIdentifier;
use buttplug_server::{
  message::{ButtplugClientMessageVariant, ButtplugServerMessageVariant}, ButtplugServer, ButtplugServerBuilder
};
use futures::{future::Future, pin_mut, select, FutureExt, Stream, StreamExt};
use getset::Getters;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast::{self, Sender}, mpsc, Notify};

// Clone derived here to satisfy tokio broadcast requirements.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ButtplugRemoteServerEvent {
  ClientConnected(String),
  ClientDisconnected,
  DeviceAdded {
    index: u32,
    identifier: UserDeviceIdentifier,
    name: String,
    display_name: Option<String>,
  },
  DeviceRemoved {
    index: u32,
  },
  //DeviceCommand(ButtplugDeviceCommandMessageUnion)
}

#[derive(Error, Debug)]
pub enum ButtplugServerConnectorError {
  #[error("Cannot bring up server for connection: {0}")]
  ConnectorError(String),
}

#[derive(Getters)]
pub struct ButtplugRemoteServer {
  #[getset(get = "pub")]
  server: Arc<ButtplugServer>,
  #[getset(get = "pub")]
  event_sender: broadcast::Sender<ButtplugRemoteServerEvent>,
  disconnect_notifier: Arc<Notify>,
}

async fn run_device_event_stream(
  server: Arc<ButtplugServer>,
  remote_event_sender: broadcast::Sender<ButtplugRemoteServerEvent>,
) {
  let server_receiver = server.server_version_event_stream();
  pin_mut!(server_receiver);
  loop {
    match server_receiver.next().await {
      None => {
        info!("Server disconnected via server disappearance, exiting loop.");
        break;
      }
      Some(msg) => {
        if remote_event_sender.receiver_count() > 0 {
          match &msg {
            ButtplugServerMessageV4::DeviceAdded(da) => {
              if let Some(device_info) = server.device_manager().device_info(da.device_index()) {
                let added_event = ButtplugRemoteServerEvent::DeviceAdded {
                  index: da.device_index(),
                  name: da.device_name().clone(),
                  identifier: device_info.identifier().clone().into(),
                  display_name: device_info.display_name().clone(),
                };
                if remote_event_sender.send(added_event).is_err() {
                  error!("Cannot send event to owner, dropping and assuming local server thread has exited.");
                }
              }
            }
            ButtplugServerMessageV4::DeviceRemoved(dr) => {
              let removed_event = ButtplugRemoteServerEvent::DeviceRemoved {
                index: dr.device_index(),
              };
              if remote_event_sender.send(removed_event).is_err() {
                error!("Cannot send event to owner, dropping and assuming local server thread has exited.");
              }
            }
            _ => {}
          }
        }
      }
    }
  }
}

async fn run_server<ConnectorType>(
  server: Arc<ButtplugServer>,
  remote_event_sender: broadcast::Sender<ButtplugRemoteServerEvent>,
  connector: ConnectorType,
  mut connector_receiver: mpsc::Receiver<ButtplugClientMessageVariant>,
  disconnect_notifier: Arc<Notify>,
) where
  ConnectorType:
    ButtplugConnector<ButtplugServerMessageVariant, ButtplugClientMessageVariant> + 'static,
{
  info!("Starting remote server loop");
  let shared_connector = Arc::new(connector);
  let server_receiver = server.server_version_event_stream();
  let client_version_receiver = server.event_stream();
  pin_mut!(server_receiver);
  pin_mut!(client_version_receiver);
  loop {
    select! {
      connector_msg = connector_receiver.recv().fuse() => match connector_msg {
        None => {
          info!("Connector disconnected, exiting loop.");
          if remote_event_sender.receiver_count() > 0 && remote_event_sender.send(ButtplugRemoteServerEvent::ClientDisconnected).is_err() {
            warn!("Cannot update remote about client disconnection");
          }
          break;
        }
        Some(client_message) => {
          trace!("Got message from connector: {:?}", client_message);
          let server_clone = server.clone();
          let connected = server_clone.connected();
          let connector_clone = shared_connector.clone();
          let remote_event_sender_clone = remote_event_sender.clone();
          async_manager::spawn(async move {
            match server_clone.parse_message(client_message.clone()).await {
              Ok(ret_msg) => {
                // Only send event if we just connected. Sucks to check it on every message but the boolean check should be quick.
                if !connected && server_clone.connected() {
                  if remote_event_sender_clone.receiver_count() > 0 {
                    if remote_event_sender_clone.send(ButtplugRemoteServerEvent::ClientConnected(server_clone.client_name().unwrap_or("Buttplug Client (No name specified)".to_owned()).clone())).is_err() {
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
              ButtplugServerMessageV4::DeviceAdded(da) => {
                if let Some(device_info) = server.device_manager().device_info(da.device_index()) {
                  let added_event = ButtplugRemoteServerEvent::DeviceAdded { index: da.device_index(), name: da.device_name().clone(), identifier: device_info.identifier().clone().into(), display_name: device_info.display_name().clone() };
                  if remote_event_sender.send(added_event).is_err() {
                    error!("Cannot send event to owner, dropping and assuming local server thread has exited.");
                  }
                }
              },
              ButtplugServerMessageV4::DeviceRemoved(dr) => {
                let removed_event = ButtplugRemoteServerEvent::DeviceRemoved { index: dr.device_index() };
                if remote_event_sender.send(removed_event).is_err() {
                  error!("Cannot send event to owner, dropping and assuming local server thread has exited.");
                }
              },
              _ => {}
            }
          }
        }
      },
      client_msg = client_version_receiver.next().fuse() => match client_msg {
        None => {
          info!("Server disconnected via server disappearance, exiting loop.");
          break;
        }
        Some(msg) => {
          let connector_clone = shared_connector.clone();
          if connector_clone.send(msg.into()).await.is_err() {
            error!("Server disappeared, exiting remote server thread.");
          }
        }
      }
    };
  }
  if let Err(err) = server.disconnect().await {
    error!("Error disconnecting server: {:?}", err);
  }
  info!("Exiting remote server loop");
}

impl Default for ButtplugRemoteServer {
  fn default() -> Self {
    Self::new(
      ButtplugServerBuilder::default()
        .finish()
        .expect("Default is infallible"),
      &None
    )
  }
}

impl ButtplugRemoteServer {
  pub fn new(server: ButtplugServer, event_sender: &Option<Sender<ButtplugRemoteServerEvent>>) -> Self {
    let event_sender = if let Some(sender) = event_sender {
      sender.clone()
    } else {
      broadcast::channel(256).0
    };
    // Thanks to the existence of the backdoor server, device updates can happen for the lifetime to
    // the RemoteServer instance, not just during client connect. We need to make sure these are
    // emitted to the frontend.
    let server = Arc::new(server);
    {
      let server = server.clone();
      tokio::spawn({
        let server = server;
        let event_sender = event_sender.clone();
        async move {
          run_device_event_stream(server, event_sender).await;
        }
      });
    }
    Self {
      event_sender,
      server: server,
      disconnect_notifier: Arc::new(Notify::new()),
    }
  }

  pub fn event_stream(&self) -> impl Stream<Item = ButtplugRemoteServerEvent> + use<> {
    convert_broadcast_receiver_to_stream(self.event_sender.subscribe())
  }

  pub fn start<ConnectorType>(
    &self,
    mut connector: ConnectorType,
  ) -> impl Future<Output = Result<(), ButtplugServerConnectorError>> + use<ConnectorType>
  where
    ConnectorType:
      ButtplugConnector<ButtplugServerMessageVariant, ButtplugClientMessageVariant> + 'static,
  {
    let server = self.server.clone();
    let event_sender = self.event_sender.clone();
    let disconnect_notifier = self.disconnect_notifier.clone();
    async move {
      let (connector_sender, connector_receiver) = mpsc::channel(256);
      // Due to the connect method requiring a mutable connector, we must connect before starting up
      // our server loop. Anything that needs to happen outside of the client connection session
      // should happen around this. This flow is locked.
      connector
        .connect(connector_sender)
        .await
        .map_err(|e| ButtplugServerConnectorError::ConnectorError(format!("{:?}", e)))?;
      run_server(
        server,
        event_sender,
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

  pub async fn shutdown(&self) -> Result<(), ButtplugError> {
    self.server.shutdown().await?;
    Ok(())
  }
}

impl Drop for ButtplugRemoteServer {
  fn drop(&mut self) {
    self.disconnect_notifier.notify_waiters();
  }
}
