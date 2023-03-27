// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug::{
  core::{
    connector::ButtplugConnector,
    errors::ButtplugError,
    message::{
      self,
      ButtplugClientMessage,
      ButtplugMessage,
      ButtplugMessageValidator,
      ButtplugServerMessage,
    },
  },
  server::{ButtplugServer, ButtplugServerBuilder},
  util::async_manager,
};
use futures::{future::Future, pin_mut, select, FutureExt, StreamExt};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{mpsc, Notify};
use tracing::*;

#[derive(Error, Debug)]
pub enum ButtplugServerConnectorError {
  #[error("Cannot bring up server for connection: {0}")]
  ConnectorError(String),
}

pub struct ButtplugTestServer {
  server: Arc<ButtplugServer>,
  disconnect_notifier: Arc<Notify>,
}

async fn run_server<ConnectorType>(
  server: Arc<ButtplugServer>,
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
          async_manager::spawn(async move {
            if let Err(e) = client_message.is_valid() {
              error!("Message not valid: {:?} - Error: {}", client_message, e);
              let mut err_msg = message::Error::from(ButtplugError::from(e));
              err_msg.set_id(client_message.id());
              connector_clone.send(err_msg.into());
              return;
            }
            match server_clone.parse_message(client_message.clone()).await {
              Ok(ret_msg) => {
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
          if shared_connector.send(msg).await.is_err() {
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

impl Default for ButtplugTestServer {
  fn default() -> Self {
    Self::new(
      ButtplugServerBuilder::default()
        .finish()
        .expect("Default is infallible"),
    )
  }
}

impl ButtplugTestServer {
  pub fn new(server: ButtplugServer) -> Self {
    Self {
      server: Arc::new(server),
      disconnect_notifier: Arc::new(Notify::new()),
    }
  }

  pub fn start<ConnectorType>(
    &self,
    mut connector: ConnectorType,
  ) -> impl Future<Output = Result<(), ButtplugServerConnectorError>>
  where
    ConnectorType: ButtplugConnector<ButtplugServerMessage, ButtplugClientMessage> + 'static,
  {
    let server_clone = self.server.clone();
    let disconnect_notifier = self.disconnect_notifier.clone();
    async move {
      let (connector_sender, connector_receiver) = mpsc::channel(256);
      connector
        .connect(connector_sender)
        .await
        .map_err(|e| ButtplugServerConnectorError::ConnectorError(format!("{:?}", e)))?;
      run_server(
        server_clone,
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

impl Drop for ButtplugTestServer {
  fn drop(&mut self) {
    self.disconnect_notifier.notify_waiters();
  }
}
