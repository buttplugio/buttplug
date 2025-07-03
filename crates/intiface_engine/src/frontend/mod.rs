pub mod process_messages;
use crate::error::IntifaceError;
use crate::remote_server::ButtplugRemoteServerEvent;
use async_trait::async_trait;
use futures::{pin_mut, Stream, StreamExt};
pub use process_messages::{EngineMessage, IntifaceMessage};
use std::sync::Arc;
use tokio::{
  select,
  sync::{broadcast, Notify},
};
use tokio_util::sync::CancellationToken;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[async_trait]
pub trait Frontend: Sync + Send {
  async fn send(&self, msg: EngineMessage);
  async fn connect(&self) -> Result<(), IntifaceError>;
  fn disconnect_notifier(&self) -> Arc<Notify>;
  fn disconnect(&self);
  fn event_stream(&self) -> broadcast::Receiver<IntifaceMessage>;
}

pub async fn frontend_external_event_loop(
  frontend: Arc<dyn Frontend>,
  connection_cancellation_token: Arc<CancellationToken>,
) {
  let mut external_receiver = frontend.event_stream();
  loop {
    select! {
      external_message = external_receiver.recv() => {
        match external_message {
          Ok(message) => match message {
            IntifaceMessage::RequestEngineVersion{expected_version:_} => {
              // TODO We should check the version here and shut down on mismatch.
              info!("Engine version request received from frontend.");
              frontend
                .send(EngineMessage::EngineVersion{ version: VERSION.to_owned() })
                .await;
            },
            IntifaceMessage::Stop{} => {
              connection_cancellation_token.cancel();
              info!("Got external stop request");
              break;
            }
          },
          Err(_) => {
            info!("Frontend sender dropped, assuming connection lost, breaking.");
            break;
          }
        }
      },
      _ = connection_cancellation_token.cancelled() => {
        info!("Connection cancellation token activated, breaking from frontend external event loop.");
        break;
      }
    }
  }
}

pub async fn frontend_server_event_loop(
  receiver: impl Stream<Item = ButtplugRemoteServerEvent>,
  frontend: Arc<dyn Frontend>,
  connection_cancellation_token: CancellationToken,
) {
  pin_mut!(receiver);

  loop {
    select! {
      maybe_event = receiver.next() => {
        match maybe_event {
          Some(event) => match event {
            ButtplugRemoteServerEvent::ClientConnected(client_name) => {
              info!("Client connected: {}", client_name);
              frontend.send(EngineMessage::ClientConnected{client_name}).await;
            }
            ButtplugRemoteServerEvent::ClientDisconnected => {
              info!("Client disconnected.");
              frontend
                .send(EngineMessage::ClientDisconnected{})
                .await;
            }
            ButtplugRemoteServerEvent::DeviceAdded { index: device_id, name: device_name, identifier: device_address, display_name: device_display_name } => {
              info!("Device Added: {} - {} - {:?}", device_id, device_name, device_address);
              frontend
                .send(EngineMessage::DeviceConnected { name: device_name, index: device_id, identifier: device_address, display_name: device_display_name })
                .await;
            }
            ButtplugRemoteServerEvent::DeviceRemoved { index: device_id } => {
              info!("Device Removed: {}", device_id);
              frontend
                .send(EngineMessage::DeviceDisconnected{index: device_id})
                .await;
            }
          },
          None => {
            info!("Lost connection with main thread, breaking.");
            break;
          },
        }
      },
      _ = connection_cancellation_token.cancelled() => {
        info!("Connection cancellation token activated, breaking from frontend server event loop");
        break;
      }
    }
  }
  info!("Exiting server event receiver loop");
}
/*
#[derive(Default)]
struct NullFrontend {
  notify: Arc<Notify>,
}

#[async_trait]
impl Frontend for NullFrontend {
  async fn send(&self, _: EngineMessage) {}
  async fn connect(&self) -> Result<(), IntifaceError> {
    Ok(())
  }
  fn disconnect(&self) {
    self.notify.notify_waiters();
  }
  fn disconnect_notifier(&self) -> Arc<Notify> {
    self.notify.clone()
  }
  fn event_stream(&self) -> broadcast::Receiver<IntifaceMessage> {
    let (_, receiver) = broadcast::channel(255);
    receiver
  }
}
*/