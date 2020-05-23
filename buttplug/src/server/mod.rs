// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handles client sessions, as well as discovery and communication with hardware.

pub mod comm_managers;
pub mod device_manager;
mod logger;
pub mod wrapper;

use crate::{
  core::{
    errors::*,
    messages::{
      self,
      ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceManagerMessageUnion,
      ButtplugInMessage,
      ButtplugMessage,
      ButtplugMessageSpecVersion,
      ButtplugOutMessage,
      DeviceMessageInfo,
      StopAllDevices,
      BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
    },
  },
  test::TestDeviceImplCreator,
};
use async_std::{
  sync::{channel, Arc, Mutex, Receiver, Sender},
  task,
};
use comm_managers::{DeviceCommunicationManager, DeviceCommunicationManagerCreator};
use device_manager::DeviceManager;
use logger::ButtplugLogHandler;
use std::{
  convert::{TryFrom, TryInto},
  sync::{self, RwLock},
  time::{Duration, Instant},
};

pub enum ButtplugServerEvent {
  DeviceAdded(DeviceMessageInfo),
  DeviceRemoved(DeviceMessageInfo),
  DeviceMessage(ButtplugOutMessage),
  ScanningFinished(),
  ServerError(ButtplugError),
  PingTimeout(),
  Log(messages::Log),
}

struct PingTimer {
  // Needs to be a u128 to compare with Instant, otherwise we have to cast up.
  // This is painful either direction. See
  // https://github.com/rust-lang/rust/issues/58580
  max_ping_time: u128,
  last_ping_time: sync::Arc<RwLock<Instant>>,
  pinged_out: sync::Arc<RwLock<bool>>,
  // This should really be a Condvar but async_std::Condvar isn't done yet, so
  // we'll just use a channel. The channel receiver will get passed to the
  // device manager, so it can stop devices
  ping_channel: Sender<bool>,
  // TODO This should be an RwLock once that's in async-std
  timer_running: Arc<Mutex<bool>>,
}

impl PingTimer {
  pub fn new(max_ping_time: u128) -> (Self, Receiver<bool>) {
    if max_ping_time == 0 {
      panic!("Can't create ping timer with no max ping time.");
    }
    let (sender, receiver) = channel(1);
    (
      Self {
        max_ping_time,
        last_ping_time: Arc::new(RwLock::new(Instant::now())),
        pinged_out: Arc::new(RwLock::new(false)),
        ping_channel: sender,
        timer_running: Arc::new(Mutex::new(false)),
      },
      receiver,
    )
  }

  pub fn start_ping_timer(&mut self, event_sender: Sender<ButtplugOutMessage>) {
    // Since we've received the handshake, start the ping timer if needed.
    let max_ping_time = self.max_ping_time;
    let last_ping_time = self.last_ping_time.clone();
    let pinged_out = self.pinged_out.clone();
    let ping_channel = self.ping_channel.clone();
    let timer_running = self.timer_running.clone();
    task::spawn(async move {
      loop {
        {
          *timer_running.lock().await = true;
        }
        task::sleep(Duration::from_millis(max_ping_time.try_into().unwrap())).await;
        // If the timer is no longer supposed to be running, bail.
        if !*timer_running.lock().await {
          return;
        }
        let last_ping = last_ping_time.read().unwrap().elapsed().as_millis();
        if last_ping > max_ping_time {
          error!("Pinged out.");
          *pinged_out.write().unwrap() = true;
          ping_channel.send(true).await;
          let err: ButtplugError = ButtplugPingError::new(&format!(
            "Pinged out. Ping took {} but max ping time is {}.",
            last_ping, max_ping_time
          ))
          .into();
          event_sender
            .send(ButtplugOutMessage::Error(err.into()))
            .await;
          break;
        }
      }
    });
  }

  pub async fn stop_ping_timer(&mut self) {
    *self.timer_running.lock().await = false;
    *self.pinged_out.write().unwrap() = false;
  }

  pub fn max_ping_time(&self) -> u128 {
    self.max_ping_time
  }

  pub fn update_ping_time(&mut self) {
    *self.last_ping_time.write().unwrap() = Instant::now();
  }

  pub fn pinged_out(&self) -> bool {
    *self.pinged_out.read().unwrap()
  }
}

// TODO Impl Drop for ping timer that stops the internal async task

/// Represents a ButtplugServer.
pub struct ButtplugServer {
  server_name: String,
  server_spec_version: ButtplugMessageSpecVersion,
  client_spec_version: Option<ButtplugMessageSpecVersion>,
  client_name: Option<String>,
  device_manager: Option<DeviceManager>,
  event_sender: Sender<ButtplugOutMessage>,
  event_receiver: Receiver<ButtplugOutMessage>,
  ping_timer: Option<PingTimer>,
}

impl ButtplugServer {
  pub fn new(name: &str, max_ping_time: u128) -> (Self, Receiver<ButtplugOutMessage>) {
    let (send, recv) = channel(256);
    let (ping_timer, ping_receiver) = if max_ping_time > 0 {
      let (timer, receiver) = PingTimer::new(max_ping_time);
      (Some(timer), Some(receiver))
    } else {
      (None, None)
    };
    (
      Self {
        server_name: name.to_string(),
        server_spec_version: BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
        client_name: None,
        client_spec_version: None,
        device_manager: Some(DeviceManager::new(send.clone(), ping_receiver)),
        ping_timer,
        event_sender: send,
        event_receiver: recv.clone(),
      },
      recv,
    )
  }

  pub fn take_device_manager(&mut self) -> Option<DeviceManager> {
    self.device_manager.take()
  }

  pub fn add_comm_manager<T>(&mut self)
  where
    T: 'static + DeviceCommunicationManager + DeviceCommunicationManagerCreator,
  {
    if let Some(ref mut dm) = self.device_manager {
      dm.add_comm_manager::<T>();
    } else {
      panic!("Device Manager has been taken already!");
    }
  }

  pub fn add_test_comm_manager(&mut self) -> Arc<Mutex<Vec<TestDeviceImplCreator>>> {
    if let Some(ref mut dm) = self.device_manager {
      dm.add_test_comm_manager()
    } else {
      panic!("Device Manager has been taken already!");
    }
  }

  pub fn get_event_receiver(&self) -> Receiver<ButtplugOutMessage> {
    self.event_receiver.clone()
  }

  pub fn connected(&self) -> bool {
    self.client_name.is_some() && self.client_spec_version.is_some()
  }

  pub async fn disconnect(&mut self) {
    if let Some(ref mut ping_timer) = self.ping_timer {
      ping_timer.stop_ping_timer().await;
    }
    self
      .parse_message(&ButtplugInMessage::StopAllDevices(StopAllDevices::default()))
      .await
      .unwrap();
    self.client_name = None;
    self.client_spec_version = None;
  }

  pub async fn parse_message(
    &mut self,
    msg: &ButtplugInMessage,
  ) -> Result<ButtplugOutMessage, ButtplugError> {
    if let Some(timer) = &self.ping_timer {
      if timer.pinged_out() {
        return Err(ButtplugPingError::new("Server has pinged out.").into());
      }
    }
    if ButtplugDeviceManagerMessageUnion::try_from(msg.clone()).is_ok()
      || ButtplugDeviceCommandMessageUnion::try_from(msg.clone()).is_ok()
    {
      if !self.connected() {
        return Err(ButtplugHandshakeError::new("Server not connected.").into());
      }
      if let Some(ref mut dm) = self.device_manager {
        dm.parse_message(msg.clone()).await
      } else {
        panic!("Device Manager has been taken already!");
      }
    } else {
      match msg {
        ButtplugInMessage::RequestServerInfo(ref m) => {
          self.perform_handshake(m).and_then(|m| Ok(m.into()))
        }
        ButtplugInMessage::Ping(ref p) => {
          if !self.connected() {
            return Err(ButtplugHandshakeError::new("Server not connected.").into());
          }
          self.handle_ping(p).and_then(|m| Ok(m.into()))
        }
        ButtplugInMessage::RequestLog(ref l) => {
          if !self.connected() {
            return Err(ButtplugHandshakeError::new("Server not connected.").into());
          }
          self.handle_log(l).and_then(|m| Ok(m.into()))
        }
        _ => Err(
          ButtplugMessageError::new(&format!("Message {:?} not handled by server loop.", msg))
            .into(),
        ),
      }
    }
  }

  fn perform_handshake(
    &mut self,
    msg: &messages::RequestServerInfo,
  ) -> Result<messages::ServerInfo, ButtplugError> {
    if self.connected() {
      return Err(ButtplugHandshakeError::new("Server already connected.").into());
    }
    if self.server_spec_version < msg.message_version {
      return Err(
        ButtplugHandshakeError::new(&format!(
          "Server version ({}) must be equal to or greater than client version ({}).",
          self.server_spec_version, msg.message_version
        ))
        .into(),
      );
    }
    self.client_name = Some(msg.client_name.clone());
    self.client_spec_version = Some(msg.message_version);
    // Only start the ping timer after we've received the handshake.
    let mut max_ping_time = 0u128;
    if let Some(timer) = &mut self.ping_timer {
      max_ping_time = timer.max_ping_time();
      timer.start_ping_timer(self.event_sender.clone());
    }
    Result::Ok(messages::ServerInfo::new(
      &self.server_name,
      self.server_spec_version,
      max_ping_time.try_into().unwrap(),
    ))
  }

  fn handle_ping(&mut self, msg: &messages::Ping) -> Result<messages::Ok, ButtplugError> {
    if let Some(timer) = &mut self.ping_timer {
      timer.update_ping_time();
      Result::Ok(messages::Ok::new(msg.get_id()))
    } else {
      Err(ButtplugPingError::new("Ping message invalid, as ping timer is not running.").into())
    }
  }

  fn handle_log(&mut self, msg: &messages::RequestLog) -> Result<messages::Ok, ButtplugError> {
    let handler = ButtplugLogHandler::new(&msg.log_level, self.event_sender.clone());
    log::set_boxed_logger(Box::new(handler))
      .map_err(|e| ButtplugUnknownError::new(&format!("Cannot set up log handler: {}", e)).into())
      .and_then(|_| {
        let level: log::LevelFilter = msg.log_level.clone().into();
        log::set_max_level(level);
        Result::Ok(messages::Ok::new(msg.get_id()))
      })
  }
}

#[cfg(test)]
mod test {
  use super::*;
  #[cfg(any(feature = "linux-ble", feature = "winrt-ble"))]
  use crate::server::comm_managers::btleplug::BtlePlugCommunicationManager;
  use crate::{
    device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
    test::{check_recv_value, TestDevice},
  };
  use async_std::{prelude::StreamExt, sync::Receiver, task};
  use std::time::Duration;

  async fn test_server_setup(
    msg_union: &messages::ButtplugInMessage,
  ) -> (ButtplugServer, Receiver<ButtplugOutMessage>) {
    let _ = env_logger::builder().is_test(true).try_init();
    let (mut server, recv) = ButtplugServer::new("Test Server", 0);
    assert_eq!(server.server_name, "Test Server");
    match server.parse_message(&msg_union).await.unwrap() {
      ButtplugOutMessage::ServerInfo(_s) => assert_eq!(
        _s,
        messages::ServerInfo::new("Test Server", ButtplugMessageSpecVersion::Version2, 0)
      ),
      _ => panic!("Should've received ok"),
    }
    (server, recv)
  }

  #[test]
  fn test_server_handshake() {
    let _ = env_logger::builder().is_test(true).try_init();
    let msg =
      messages::RequestServerInfo::new("Test Client", ButtplugMessageSpecVersion::Version2).into();
    task::block_on(async {
      let (server, _recv) = test_server_setup(&msg).await;
      assert_eq!(server.client_name.unwrap(), "Test Client");
    });
  }

  #[test]
  fn test_server_version_lt() {
    let _ = env_logger::builder().is_test(true).try_init();
    let msg =
      messages::RequestServerInfo::new("Test Client", ButtplugMessageSpecVersion::Version2).into();
    task::block_on(async {
      test_server_setup(&msg).await;
    });
  }

  // TODO Now that we're moving to a spec version enum, this test is invalid
  // because we can't just pass a u8 in. This should be rebuilt using the
  // JSON parser, and it should fail to deserialize the message.
  #[test]
  #[ignore]
  fn test_server_version_gt() {
    let _ = env_logger::builder().is_test(true).try_init();
    let (mut server, _) = ButtplugServer::new("Test Server", 0);
    let msg =
      messages::RequestServerInfo::new("Test Client", ButtplugMessageSpecVersion::Version2).into();
    task::block_on(async {
      assert!(
        server.parse_message(&msg).await.is_err(),
        "Client having higher version than server should fail"
      );
    });
  }

  #[test]
  fn test_ping_timeout() {
    let _ = env_logger::builder().is_test(true).try_init();
    let (mut server, mut recv) = ButtplugServer::new("Test Server", 100);
    task::block_on(async {
      let msg = messages::RequestServerInfo::new("Test Client", server.server_spec_version);
      task::sleep(Duration::from_millis(150)).await;
      let reply = server.parse_message(&msg.into()).await;
      assert!(
        reply.is_ok(),
        format!(
          "ping timer shouldn't start until handshake finished. {:?}",
          reply
        )
      );
      task::sleep(Duration::from_millis(300)).await;
      let pingmsg = messages::Ping::default();
      match server.parse_message(&pingmsg.into()).await {
        Ok(_) => panic!("Should get a ping error back!"),
        Err(e) => {
          if let ButtplugError::ButtplugPingError(_) = e {
            // do nothing
          } else {
            panic!("Got wrong type of error back!");
          }
        }
      }
      // Check that we got an event back about the ping out.
      let msg = recv.next().await.unwrap();
      if let ButtplugOutMessage::Error(e) = msg {
        if let ButtplugError::ButtplugPingError(_) = e.into() {
        } else {
          panic!("Didn't get a ping error");
        }
      } else {
        panic!("Didn't get an error message back");
      }
    });
  }

  #[test]
  #[ignore]
  fn test_device_stop_on_ping_timeout() {
    let _ = env_logger::builder().is_test(true).try_init();

    task::block_on(async {
      let (mut server, mut recv) = ButtplugServer::new("Test Server", 100);
      let devices = server.add_test_comm_manager();
      // TODO This should probably use a test protocol we control, not the aneros protocol
      let (device, device_creator) =
        TestDevice::new_bluetoothle_test_device_impl_creator("Massage Demo");
      devices.lock().await.push(device_creator);

      let msg = messages::RequestServerInfo::new("Test Client", server.server_spec_version);
      let mut reply = server.parse_message(&msg.into()).await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      reply = server
        .parse_message(&messages::StartScanning::default().into())
        .await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      // Check that we got an event back about a new device.
      let msg = recv.next().await.unwrap();
      if let ButtplugOutMessage::DeviceAdded(da) = msg {
        assert_eq!(da.device_name, "Aneros Vivi");
      } else {
        panic!(format!(
          "Returned message was not a DeviceAdded message or timed out: {:?}",
          msg
        ));
      }
      server
        .parse_message(
          &messages::VibrateCmd::new(0, vec![messages::VibrateSubcommand::new(0, 0.5)]).into(),
        )
        .await
        .unwrap();
      let (_, command_receiver) = device.get_endpoint_channel_clone(Endpoint::Tx).await;
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 63], false)),
      )
      .await;
      // Wait out the ping, we should get a stop message.
      let mut i = 0u32;
      while command_receiver.is_empty() {
        task::sleep(Duration::from_millis(150)).await;
        // Breaks out of loop if we wait for too long.
        i += 1;
        assert!(i < 10, "Slept for too long while waiting for stop command!");
      }
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 0], false)),
      )
      .await;
    });
  }

  // Warning: This test is brittle. If any log messages are fired between our
  // log in this message and the asserts, it will fail. If you see failures on
  // this test, that's probably why.
  #[test]
  #[ignore]
  fn test_log_handler() {
    // The log crate only allows one log handler at a time, meaning if we
    // set up env_logger, our server log function won't work. This is a
    // problem. Only uncomment this if this test if failing and you need to
    // see output.
    //
    // let _ = env_logger::builder().is_test(true).try_init();
    let (mut server, mut recv) = ButtplugServer::new("Test Server", 0);
    task::block_on(async {
      let msg = messages::RequestServerInfo::new("Test Client", server.server_spec_version);
      let mut reply = server.parse_message(&msg.into()).await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      reply = server
        .parse_message(&messages::RequestLog::new(messages::LogLevel::Debug).into())
        .await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      debug!("Test log message");

      let mut did_log = false;
      // Check that we got an event back about a new device.

      while let Some(msg) = recv.next().await {
        if let ButtplugOutMessage::Log(log) = msg {
          // We can't assert here, because we may get multiple log
          // messages back, so we just want to break whenever we get
          // what we expected.
          assert_eq!(log.log_level, messages::LogLevel::Debug);
          assert!(log.log_message.contains("Test log message"));
          did_log = true;
          break;
        }
      }

      assert!(did_log, "Should've gotten log message");
    });
  }
}
