// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handles client sessions, as well as discovery and communication with hardware.

pub mod comm_managers;
pub mod device_manager;
mod ping_timer;
mod device_manager_event_loop;
pub mod remote_server;

pub use remote_server::ButtplugRemoteServer;

use crate::{core::{
    errors::*,
    messages::{
      self,
      ButtplugClientMessage,
      ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceManagerMessageUnion,
      ButtplugMessage,
      ButtplugServerMessage,
      StopAllDevices,
      StopScanning,
      BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
    },
  }, test::TestDeviceCommunicationManagerHelper, util::{async_manager, stream::convert_broadcast_receiver_to_stream}};
use tokio::sync::broadcast;
use comm_managers::{DeviceCommunicationManager, DeviceCommunicationManagerCreator};
use device_manager::DeviceManager;
use futures::{future::BoxFuture, Stream, StreamExt};
use ping_timer::PingTimer;
use std::{
  convert::{TryFrom, TryInto},
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use thiserror::Error;

pub type ButtplugServerResult = Result<ButtplugServerMessage, ButtplugError>;
pub type ButtplugServerResultFuture = BoxFuture<'static, ButtplugServerResult>;

#[derive(Error, Debug)]
pub enum ButtplugServerStartupError {
  #[error("DeviceManager of type {0} has already been added.")]
  DeviceManagerTypeAlreadyAdded(String),
}

#[derive(Debug, Clone)]
pub struct ButtplugServerOptions {
  pub name: String,
  pub max_ping_time: u64,
  pub allow_raw_messages: bool,
  pub device_configuration_json: Option<String>,
  pub user_device_configuration_json: Option<String>,
}

impl Default for ButtplugServerOptions {
  fn default() -> Self {
    Self {
      name: "Buttplug Server".to_owned(),
      max_ping_time: 0,
      allow_raw_messages: false,
      device_configuration_json: None,
      user_device_configuration_json: None,
    }
  }
}

/// Represents a ButtplugServer.
pub struct ButtplugServer {
  server_name: String,
  client_name: String,
  max_ping_time: u64,
  device_manager: DeviceManager,
  ping_timer: Arc<PingTimer>,
  pinged_out: Arc<AtomicBool>,
  connected: Arc<AtomicBool>,
  output_sender: broadcast::Sender<ButtplugServerMessage>
}

impl Default for ButtplugServer {
  fn default() -> Self {
    // We can unwrap here because if default init fails, so will pretty much every test.
    Self::new_with_options(&ButtplugServerOptions::default()).unwrap()
  }
}

impl ButtplugServer {
  pub fn new_with_options(
    options: &ButtplugServerOptions,
  ) -> Result<Self, ButtplugError> {
    let (send, _) = broadcast::channel(256);
    let output_sender_clone = send.clone();
    let pinged_out = Arc::new(AtomicBool::new(false));
    let connected = Arc::new(AtomicBool::new(false));
      let ping_timer = Arc::new(PingTimer::new(options.max_ping_time));
      let ping_timeout_notifier = ping_timer.ping_timeout_waiter();
      let connected_clone = connected.clone();
      let event_sender_clone = send.clone();
      async_manager::spawn(async move {
        // This will only exit if we've pinged out.
        ping_timeout_notifier.await;
        error!("Ping out signal received, stopping server");
        connected_clone.store(false, Ordering::SeqCst);
        // TODO Should the event sender return a result instead of an error message?
        if output_sender_clone
          .send(messages::Error::new(messages::ErrorCode::ErrorPing, "Ping Timeout").into())
          .is_err()
        {
          error!("Server disappeared, cannot update about ping out.");
        };
      })
      .unwrap();
    let device_manager = DeviceManager::try_new(
      send.clone(),
      ping_timer.clone(),
      options.allow_raw_messages,
      &options.device_configuration_json,
      &options.user_device_configuration_json,
    )?;
    Ok(Self {
      server_name: options.name.clone(),
      client_name: String::default(),
      max_ping_time: options.max_ping_time,
      device_manager,
      ping_timer,
      pinged_out,
      connected,
      output_sender: send
    })
  }

  pub fn event_stream(&self) -> impl Stream<Item = ButtplugServerMessage> {
    // Unlike the client API, we can expect anyone using the server to pin this
    // themselves.
    convert_broadcast_receiver_to_stream(self.output_sender.subscribe())
  }

  pub fn client_name(&self) -> String {
    self.client_name.clone()
  }

  pub fn add_comm_manager<T>(&self) -> Result<(), ButtplugServerStartupError>
  where
    T: 'static + DeviceCommunicationManager + DeviceCommunicationManagerCreator,
  {
    self.device_manager.add_comm_manager::<T>()
  }

  pub fn add_test_comm_manager(
    &self,
  ) -> Result<TestDeviceCommunicationManagerHelper, ButtplugServerStartupError> {
    self.device_manager.add_test_comm_manager()
  }

  pub fn connected(&self) -> bool {
    self.connected.load(Ordering::SeqCst)
  }

  pub fn disconnect(&self) -> BoxFuture<Result<(), ButtplugServerError>> {
    let ping_timer = self.ping_timer.clone();
    let stop_scanning_fut =
      self.parse_message(ButtplugClientMessage::StopScanning(StopScanning::default()));
    let stop_fut = self.parse_message(ButtplugClientMessage::StopAllDevices(
      StopAllDevices::default(),
    ));
    let connected = self.connected.clone();
    Box::pin(async move {
      // TODO We should really log more here.
      connected.store(false, Ordering::SeqCst);
      ping_timer.stop_ping_timer();
      // Ignore returns here, we just want to stop.
      info!("Server disconnected, stopping all devices...");
      let _ = stop_fut.await;
      info!("Server disconnected, stopping device scanning if it was started...");
      let _ = stop_scanning_fut.await;
      Ok(())
    })
  }

  // This is the only method that returns ButtplugServerResult, as it handles
  // the packing of the message ID.
  pub fn parse_message(
    &self,
    msg: ButtplugClientMessage,
  ) -> BoxFuture<'static, Result<ButtplugServerMessage, ButtplugServerError>> {
    let id = msg.get_id();
    if !self.connected() {
      // Check for ping timeout first! There's no way we should've pinged out if
      // we haven't received RequestServerInfo first, but we do want to know if
      // we pinged out.
      if self.pinged_out.load(Ordering::SeqCst) {
        return ButtplugServerError::new_message_error(
          msg.get_id(),
          ButtplugPingError::PingedOut.into(),
        )
        .into();
      } else if !matches!(msg, ButtplugClientMessage::RequestServerInfo(_)) {
        return ButtplugServerError::from(ButtplugHandshakeError::RequestServerInfoExpected).into();
      }
    }
    // Produce whatever future is needed to reply to the message, this may be a
    // device command future, or something the server handles. All futures will
    // return Result<ButtplugServerMessage, ButtplugError>, and we'll handle
    // tagging the result with the message id in the future we put out as the
    // return value from this method.
    let out_fut = if ButtplugDeviceManagerMessageUnion::try_from(msg.clone()).is_ok()
      || ButtplugDeviceCommandMessageUnion::try_from(msg.clone()).is_ok()
    {
      self.device_manager.parse_message(msg.clone())
    } else {
      match msg {
        ButtplugClientMessage::RequestServerInfo(rsi_msg) => self.perform_handshake(rsi_msg),
        ButtplugClientMessage::Ping(p) => self.handle_ping(p),
        _ => ButtplugMessageError::UnexpectedMessageType(format!("{:?}", msg)).into(),
      }
    };
    // Simple way to set the ID on the way out. Just rewrap
    // the returned future to make sure it happens.
    Box::pin(async move {
      out_fut
        .await
        .map(|mut ok_msg| {
          ok_msg.set_id(id);
          ok_msg
        })
        .map_err(|err| ButtplugServerError::new_message_error(id, err))
    })
  }

  fn perform_handshake(&self, msg: messages::RequestServerInfo) -> ButtplugServerResultFuture {
    if self.connected() {
      return ButtplugHandshakeError::HandshakeAlreadyHappened.into();
    }
    if BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION < msg.message_version {
      return ButtplugHandshakeError::MessageSpecVersionMismatch(
        BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
        msg.message_version,
      )
      .into();
    }
    info!("Performing server handshake check");
    // self.client_name = Some(msg.client_name.clone());
    // self.client_spec_version = Some(msg.message_version);
    // Only start the ping timer after we've received the handshake.
    let ping_timer = self.ping_timer.clone();
    let out_msg = messages::ServerInfo::new(
      &self.server_name,
      BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
      self.max_ping_time.try_into().unwrap(),
    );
    let connected = self.connected.clone();
    Box::pin(async move {
      ping_timer.start_ping_timer();
      connected.store(true, Ordering::SeqCst);
      info!("Server handshake check successful.");
      Result::Ok(out_msg.into())
    })
  }

  fn handle_ping(&self, msg: messages::Ping) -> ButtplugServerResultFuture {
    if self.max_ping_time == 0 {
      return ButtplugPingError::PingTimerNotRunning.into();
    }
    let fut = self.ping_timer.update_ping_time();
    Box::pin(async move {
      fut.await;
      Result::Ok(messages::Ok::new(msg.get_id()).into())
    })
  }
}

#[cfg(test)]
mod test {
  use crate::{
    core::messages::{self, BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION},
    server::ButtplugServer,
    util::async_manager,
  };

  #[test]
  fn test_server_reuse() {
    let server = ButtplugServer::default();
    async_manager::block_on(async {
      let msg =
        messages::RequestServerInfo::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION);
      let mut reply = server.parse_message(msg.clone().into()).await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      reply = server.parse_message(msg.clone().into()).await;
      assert!(
        reply.is_err(),
        format!("Should get back err on double handshake: {:?}", reply)
      );
      assert!(
        server.disconnect().await.is_ok(),
        format!("Should disconnect ok")
      );
      reply = server.parse_message(msg.clone().into()).await;
      assert!(
        reply.is_ok(),
        format!(
          "Should get back ok on handshake after disconnect: {:?}",
          reply
        )
      );
    });
  }
}
