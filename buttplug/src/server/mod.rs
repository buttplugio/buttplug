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
pub mod remote_server;

pub use remote_server::ButtplugRemoteServer;

use crate::{
  core::{
    errors::*,
    messages::{
      self,
      ButtplugClientMessage,
      ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceManagerMessageUnion,
      ButtplugMessage,
      ButtplugServerMessage,
      DeviceMessageInfo,
      StopAllDevices,
      BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
    },
  },
  test::TestDeviceCommunicationManagerHelper,
  util::async_manager,
};
use async_channel::{bounded, Receiver, Sender};
use comm_managers::{DeviceCommunicationManager, DeviceCommunicationManagerCreator};
use device_manager::DeviceManager;
use futures::{future::BoxFuture, StreamExt};
use logger::ButtplugLogHandler;
use ping_timer::PingTimer;
use std::{
  convert::{TryFrom, TryInto},
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
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
  device_manager: DeviceManager,
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
          .is_err()
        {
          error!("Server disappeared, cannot update about ping out.");
        };
        if device_manager_sender.send(()).await.is_err() {
          error!("Device Manager disappeared, cannot update about ping out.");
        }
      })
      .unwrap();
      (Some(timer), Some(device_manager_receiver))
    } else {
      (None, None)
    };
    (
      Self {
        server_name: name.to_string(),
        max_ping_time,
        device_manager: DeviceManager::new(send.clone(), ping_receiver),
        ping_timer,
        pinged_out,
        connected,
        event_sender: send,
      },
      recv,
    )
  }

  pub fn add_comm_manager<T>(&self)
  where
    T: 'static + DeviceCommunicationManager + DeviceCommunicationManagerCreator,
  {
    self.device_manager.add_comm_manager::<T>();
  }

  pub fn add_test_comm_manager(&self) -> TestDeviceCommunicationManagerHelper {
    self.device_manager.add_test_comm_manager()
  }

  pub fn connected(&self) -> bool {
    self.connected.load(Ordering::SeqCst)
  }

  pub fn disconnect(&self) -> BoxFuture<Result<(), ButtplugServerError>> {
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
      stop_fut.await.map(|_| ())
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
        ButtplugClientMessage::RequestLog(l) => self.handle_log(l),
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
      info!("Server handshake check successful.");
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
      ButtplugPingError::PingTimerNotRunning.into()
    }
  }

  fn handle_log(&self, msg: messages::RequestLog) -> ButtplugServerResultFuture {
    // TODO Reimplement logging!

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
  use crate::{
    core::messages::{self, ButtplugServerMessage, BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION},
    server::ButtplugServer,
    util::async_manager,
  };
  use futures::StreamExt;

  #[test]
  fn test_server_reuse() {
    let (server, _) = ButtplugServer::new("Test Server", 0);
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
