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
mod ping_timer;

use crate::{
  core::{
    errors::*,
    messages::{
      self, ButtplugClientMessage, ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceManagerMessageUnion, ButtplugMessage, ButtplugServerMessage, DeviceMessageInfo,
      StopAllDevices, BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
    },
  },
  test::TestDeviceCommunicationManagerHelper,
  util::async_manager,
};
use futures::{StreamExt, future::BoxFuture};
use ping_timer::PingTimer;
use async_channel::{bounded, Sender, Receiver};
use comm_managers::{DeviceCommunicationManager, DeviceCommunicationManagerCreator};
use device_manager::DeviceManager;
use logger::ButtplugLogHandler;
use std::{
  convert::{TryFrom, TryInto},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  }
};

pub type ButtplugServerResult = Result<ButtplugServerMessage, ButtplugError>;
pub type ButtplugServerResultFuture = BoxFuture<'static, ButtplugServerResult>;

pub enum ButtplugServerEvent {
  DeviceAdded(DeviceMessageInfo),
  DeviceRemoved(DeviceMessageInfo),
  DeviceMessage(ButtplugServerMessage),
  ScanningFinished(),
  ServerError(ButtplugError),
  PingTimeout(),
  Log(messages::Log),
}

/// Represents a ButtplugServer.
pub struct ButtplugServer {
  server_name: String,
  max_ping_time: u64,
  device_manager: Option<DeviceManager>,
  event_sender: Sender<ButtplugServerMessage>,
  ping_timer: Option<PingTimer>,
  pinged_out: Arc<AtomicBool>,
  connected: Arc<AtomicBool>,
}

impl ButtplugServer {
  pub fn new(name: &str, max_ping_time: u64) -> (Self, Receiver<ButtplugServerMessage>) {
    let (send, recv) = bounded(256);
    let pinged_out = Arc::new(AtomicBool::new(false));
    let connected = Arc::new(AtomicBool::new(false));
    let (ping_timer, ping_receiver) = if max_ping_time > 0 {
      let (timer, mut receiver) = PingTimer::new(max_ping_time);
      // This is super dumb, but: we have a chain of channels to make sure we
      // notify both the server and the device manager. Should probably just use
      // a broadcaster here too.
      //
      // TODO Use a broadcaster here. Or just come up with a better solution.
      let (device_manager_sender, device_manager_receiver) = bounded(1);
      let pinged_out_clone = pinged_out.clone();
      let connected_clone = connected.clone();
      let event_sender_clone = send.clone();
      async_manager::spawn(async move {
        // If we receive anything here, it means we've pinged out.
        receiver.next().await;
        error!("Ping out signal received, stopping server");
        pinged_out_clone.store(true, Ordering::SeqCst);
        connected_clone.store(false, Ordering::SeqCst);
        // TODO Should the event sender return a result instead of an error message?
        if event_sender_clone
          .send(messages::Error::new(messages::ErrorCode::ErrorPing, "Ping Timeout").into())
          .await
          .is_err() {
          error!("Server disappeared, cannot update about ping out.");
        };
        if device_manager_sender.send(()).await.is_err() {
          error!("Device Manager disappeared, cannot update about ping out.");
        }
      }).unwrap();
      (Some(timer), Some(device_manager_receiver))
    } else {
      (None, None)
    };
    (
      Self {
        server_name: name.to_string(),
        max_ping_time,
        device_manager: Some(DeviceManager::new(send.clone(), ping_receiver)),
        ping_timer,
        pinged_out,
        connected,
        event_sender: send,
      },
      recv,
    )
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

  pub fn add_test_comm_manager(&mut self) -> TestDeviceCommunicationManagerHelper {
    if let Some(ref mut dm) = self.device_manager {
      dm.add_test_comm_manager()
    } else {
      panic!("Device Manager has been taken already!");
    }
  }

  pub fn connected(&self) -> bool {
    self.connected.load(Ordering::SeqCst)
  }

  pub fn disconnect(&self) -> BoxFuture<Result<(), ButtplugError>> {
    let mut ping_fut = None;
    if let Some(ping_timer) = &self.ping_timer {
      ping_fut = Some(ping_timer.stop_ping_timer());
    }
    let stop_fut = self.parse_message(ButtplugClientMessage::StopAllDevices(
      StopAllDevices::default(),
    ));
    let connected = self.connected.clone();
    Box::pin(async move {
      connected.store(false, Ordering::SeqCst);
      if let Some(pfut) = ping_fut {
        pfut.await;
      }
      stop_fut.await.and_then(|_| Ok(()))
    })
  }

  pub fn parse_message(&self, msg: ButtplugClientMessage) -> ButtplugServerResultFuture {
    let id = msg.get_id();
    let out_fut = if self.pinged_out.load(Ordering::SeqCst) {
      ButtplugPingError::new("Server has pinged out.").into()
    } else if ButtplugDeviceManagerMessageUnion::try_from(msg.clone()).is_ok()
      || ButtplugDeviceCommandMessageUnion::try_from(msg.clone()).is_ok()
    {
      if !self.connected() {
        ButtplugHandshakeError::new("Server not connected.").into()
      } else if let Some(ref dm) = self.device_manager {
        dm.parse_message(msg.clone())
      } else {
        panic!("Device Manager has been taken already!");
      }
    } else {
      match msg {
        ButtplugClientMessage::RequestServerInfo(m) => self.perform_handshake(m),
        ButtplugClientMessage::Ping(p) => {
          if !self.connected() {
            ButtplugHandshakeError::new("Server not connected.").into()
          } else {
            self.handle_ping(p)
          }
        }
        ButtplugClientMessage::RequestLog(l) => {
          if !self.connected() {
            ButtplugHandshakeError::new("Server not connected.").into()
          } else {
            self.handle_log(l)
          }
        }
        _ => ButtplugMessageError::new(&format!("Message {:?} not handled by server loop.", msg))
          .into(),
      }
    };
    // Simple way to set the ID on the way out. Just rewrap
    // the returned future to make sure it happens.
    Box::pin(async move {
      out_fut.await.and_then(|mut ok_msg| {
        ok_msg.set_id(id);
        Ok(ok_msg)
      })
    })
  }

  fn perform_handshake(&self, msg: messages::RequestServerInfo) -> ButtplugServerResultFuture {
    if self.connected() {
      return ButtplugHandshakeError::new("Server already connected.").into();
    }
    if BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION < msg.message_version {
      return ButtplugHandshakeError::new(&format!(
        "Server version ({}) must be equal to or greater than client version ({}).",
        BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION, msg.message_version
      ))
      .into();
    }
    // self.client_name = Some(msg.client_name.clone());
    // self.client_spec_version = Some(msg.message_version);
    let mut ping_timer_fut = None;
    // Only start the ping timer after we've received the handshake.
    if let Some(timer) = &self.ping_timer {
      ping_timer_fut = Some(timer.start_ping_timer());
    }
    let out_msg = messages::ServerInfo::new(
      &self.server_name,
      BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
      self.max_ping_time.try_into().unwrap(),
    );
    let connected = self.connected.clone();
    Box::pin(async move {
      if let Some(fut) = ping_timer_fut {
        fut.await;
      }
      connected.store(true, Ordering::SeqCst);
      Result::Ok(out_msg.into())
    })
  }

  fn handle_ping(&self, msg: messages::Ping) -> ButtplugServerResultFuture {
    if let Some(timer) = &self.ping_timer {
      let fut = timer.update_ping_time();
      Box::pin(async move {
        fut.await;
        Result::Ok(messages::Ok::new(msg.get_id()).into())
      })
    } else {
      ButtplugPingError::new("Ping message invalid, as ping timer is not running.").into()
    }
  }

  fn handle_log(&self, msg: messages::RequestLog) -> ButtplugServerResultFuture {
    // let sender = self.event_sender.clone();
    Box::pin(async move {
      // let handler = ButtplugLogHandler::new(&msg.log_level, sender);
      Result::Ok(messages::Ok::new(msg.get_id()).into())
    })
  }

  pub fn create_tracing_layer(&self) -> ButtplugLogHandler {
    ButtplugLogHandler::new(&messages::LogLevel::Off, self.event_sender.clone())
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::{
    core::messages::ButtplugMessageSpecVersion,
    device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
    test::check_recv_value,
    util::async_manager,
  };
  use futures::StreamExt;
  use futures_timer::Delay;
  use async_channel::Receiver;
  use std::time::Duration;

  async fn test_server_setup(
    msg_union: messages::ButtplugClientMessage,
  ) -> (ButtplugServer, Receiver<ButtplugServerMessage>) {
    let (server, recv) = ButtplugServer::new("Test Server", 0);
    assert_eq!(server.server_name, "Test Server");
    match server.parse_message(msg_union).await.unwrap() {
      ButtplugServerMessage::ServerInfo(_s) => assert_eq!(
        _s,
        messages::ServerInfo::new("Test Server", ButtplugMessageSpecVersion::Version2, 0)
      ),
      _ => panic!("Should've received ok"),
    }
    (server, recv)
  }

  #[test]
  fn test_server_handshake() {
    let msg =
      messages::RequestServerInfo::new("Test Client", ButtplugMessageSpecVersion::Version2).into();
    async_manager::block_on(async {
      let (server, _recv) = test_server_setup(msg).await;
      assert!(server.connected());
    });
  }

  #[test]
  fn test_server_version_lt() {
    let msg =
      messages::RequestServerInfo::new("Test Client", ButtplugMessageSpecVersion::Version2).into();
      async_manager::block_on(async {
      test_server_setup(msg).await;
    });
  }

  // TODO Now that we're moving to a spec version enum, this test is invalid
  // because we can't just pass a u8 in. This should be rebuilt using the
  // JSON parser, and it should fail to deserialize the message.
  #[test]
  #[ignore]
  fn test_server_version_gt() {
    let (server, _) = ButtplugServer::new("Test Server", 0);
    let msg =
      messages::RequestServerInfo::new("Test Client", ButtplugMessageSpecVersion::Version2).into();
      async_manager::block_on(async {
      assert!(
        server.parse_message(msg).await.is_err(),
        "Client having higher version than server should fail"
      );
    });
  }

  #[test]
  fn test_ping_timeout() {
    let (server, mut recv) = ButtplugServer::new("Test Server", 100);
    async_manager::block_on(async {
      let msg =
        messages::RequestServerInfo::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION);
      Delay::new(Duration::from_millis(150)).await;
      let reply = server.parse_message(msg.into()).await;
      assert!(
        reply.is_ok(),
        format!(
          "ping timer shouldn't start until handshake finished. {:?}",
          reply
        )
      );
      Delay::new(Duration::from_millis(300)).await;
      let pingmsg = messages::Ping::default();
      match server.parse_message(pingmsg.into()).await {
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
      if let ButtplugServerMessage::Error(e) = msg {
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
  fn test_device_stop_on_ping_timeout() {
    async_manager::block_on(async {
      let (mut server, mut recv) = ButtplugServer::new("Test Server", 100);
      let helper = server.add_test_comm_manager();
      // TODO This should probably use a test protocol we control, not the aneros protocol
      let device = helper.add_ble_device("Massage Demo").await;

      let msg =
        messages::RequestServerInfo::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION);
      let mut reply = server.parse_message(msg.into()).await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      reply = server
        .parse_message(messages::StartScanning::default().into())
        .await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      // Check that we got an event back about a new device.
      let msg = recv.next().await.unwrap();
      let device_index;
      if let ButtplugServerMessage::DeviceAdded(da) = msg {
        assert_eq!(da.device_name, "Aneros Vivi");
        device_index = da.device_index;
        println!("{:?}", da);
      } else {
        panic!(format!(
          "Returned message was not a DeviceAdded message or timed out: {:?}",
          msg
        ));
      }
      
      server
        .parse_message(
          messages::VibrateCmd::new(device_index, vec![messages::VibrateSubcommand::new(0, 0.5)]).into(),
        )
        .await
        .unwrap();
      let command_receiver = device.get_endpoint_channel(&Endpoint::Tx).unwrap().receiver;
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 63], false)),
      )
      .await;
      // Wait out the ping, we should get a stop message.
      let mut i = 0u32;
      while command_receiver.is_empty() {
        Delay::new(Duration::from_millis(150)).await;
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
    let (server, mut recv) = ButtplugServer::new("Test Server", 0);
    async_manager::block_on(async {
      let msg =
        messages::RequestServerInfo::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION);
      let mut reply = server.parse_message(msg.into()).await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      reply = server
        .parse_message(messages::RequestLog::new(messages::LogLevel::Debug).into())
        .await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      debug!("Test log message");

      let mut did_log = false;
      // Check that we got an event back about a new device.

      while let Some(msg) = recv.next().await {
        if let ButtplugServerMessage::Log(log) = msg {
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
