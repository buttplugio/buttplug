// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{device::ServerDeviceManager, ping_timer::PingTimer, ButtplugServerResultFuture};
use crate::{
  core::{
    errors::*,
    message::{
      self,
      ButtplugClientMessageV4,
      ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceManagerMessageUnion,
      ButtplugMessage,
      ButtplugServerMessageV4,
      StopAllDevicesV0,
      StopScanningV0,
      BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
    },
  },
  util::stream::convert_broadcast_receiver_to_stream,
};
use futures::{
  future::{self, BoxFuture, FutureExt},
  Stream,
};
use std::{
  fmt,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use tokio::sync::{broadcast, RwLock};
use tokio_stream::StreamExt;
use tracing_futures::Instrument;

/// The server side of the Buttplug protocol. Frontend for connection to device management and
/// communication.
pub struct ButtplugServer {
  /// The name of the server, which is relayed to the client on connection (mostly for
  /// confirmation in UI dialogs)
  server_name: String,
  /// The maximum ping time, in milliseconds, for the server. If the server does not receive a
  /// [Ping](crate::core::messages::Ping) message in this amount of time after the handshake has
  /// succeeded, the server will automatically disconnect. If this is not called, the ping timer
  /// will not be activated.
  ///
  /// Note that this has nothing to do with communication medium specific pings, like those built
  /// into the Websocket protocol. This ping is specific to the Buttplug protocol.
  max_ping_time: u32,
  /// Timer for managing ping time tracking, if max_ping_time > 0.
  ping_timer: Arc<PingTimer>,
  /// Manages device discovery and communication.
  device_manager: Arc<ServerDeviceManager>,
  /// If true, client is currently connected to server
  connected: Arc<AtomicBool>,
  /// Broadcaster for server events. Receivers for this are handed out through the
  /// [ButtplugServer::event_stream()] method.
  output_sender: broadcast::Sender<ButtplugServerMessageV4>,
  /// Name of the connected client, assuming there is one.
  client_name: Arc<RwLock<Option<String>>>,
}

impl std::fmt::Debug for ButtplugServer {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ButtplugServer")
      .field("server_name", &self.server_name)
      .field("max_ping_time", &self.max_ping_time)
      .field("connected", &self.connected)
      .finish()
  }
}

impl ButtplugServer {
  pub(super) fn new(
    server_name: &str,
    max_ping_time: u32,
    ping_timer: Arc<PingTimer>,
    device_manager: Arc<ServerDeviceManager>,
    connected: Arc<AtomicBool>,
    output_sender: broadcast::Sender<ButtplugServerMessageV4>,
  ) -> Self {
    ButtplugServer {
      server_name: server_name.to_owned(),
      max_ping_time,
      ping_timer,
      device_manager,
      connected,
      output_sender,
      client_name: Arc::new(RwLock::new(None)),
    }
  }

  pub fn client_name(&self) -> Option<String> {
    self
      .client_name
      .try_read()
      .expect("We should never conflict on name access")
      .clone()
  }

  /// Retreive an async stream of ButtplugServerMessages. This is how the server sends out
  /// non-query-related updates to the system, including information on devices being added/removed,
  /// client disconnection, etc...
  pub fn event_stream(&self) -> impl Stream<Item = ButtplugServerMessageV4> {
    // Unlike the client API, we can expect anyone using the server to pin this
    // themselves.
    let server_receiver = convert_broadcast_receiver_to_stream(self.output_sender.subscribe());
    let device_receiver = self.device_manager.event_stream();
    device_receiver.merge(server_receiver)
  }

  /// Returns a references to the internal device manager, for handling configuration.
  pub fn device_manager(&self) -> Arc<ServerDeviceManager> {
    self.device_manager.clone()
  }

  /// If true, client is currently connected to the server.
  pub fn connected(&self) -> bool {
    self.connected.load(Ordering::SeqCst)
  }

  /// Disconnects the server from a client, if it is connected.
  pub fn disconnect(&self) -> BoxFuture<Result<(), message::ErrorV0>> {
    debug!("Buttplug Server {} disconnect requested", self.server_name);
    let ping_timer = self.ping_timer.clone();
    // HACK Injecting messages here is weird since we're never quite sure what version they should
    // be. Can we turn this into actual method calls?
    let stop_scanning_fut = self.parse_message(ButtplugClientMessageV4::StopScanning(
      StopScanningV0::default(),
    ));
    let stop_fut = self.parse_message(ButtplugClientMessageV4::StopAllDevices(
      StopAllDevicesV0::default(),
    ));
    let connected = self.connected.clone();
    let mut name = self
      .client_name
      .try_write()
      .expect("We should never conflict on name access");
    *name = None;
    async move {
      connected.store(false, Ordering::SeqCst);
      ping_timer.stop_ping_timer().await;
      // Ignore returns here, we just want to stop.
      info!("Server disconnected, stopping device scanning if it was started...");
      let _ = stop_scanning_fut.await;
      info!("Server disconnected, stopping all devices...");
      let _ = stop_fut.await;
      Ok(())
    }
    .boxed()
  }

  pub fn shutdown(&self) -> ButtplugServerResultFuture {
    let device_manager = self.device_manager.clone();
    //let disconnect_future = self.disconnect();
    async move { device_manager.shutdown().await }.boxed()
  }

  pub fn parse_message(
    &self,
    msg: ButtplugClientMessageV4,
  ) -> BoxFuture<'static, Result<ButtplugServerMessageV4, message::ErrorV0>> {
    trace!(
      "Buttplug Server {} received message to client parse: {:?}",
      self.server_name,
      msg
    );
    let id = msg.id();
    if !self.connected() {
      // Check for ping timeout first! There's no way we should've pinged out if
      // we haven't received RequestServerInfo first, but we do want to know if
      // we pinged out.
      let error = if self.ping_timer.pinged_out() {
        Some(message::ErrorV0::from(ButtplugError::from(
          ButtplugPingError::PingedOut,
        )))
      } else if !matches!(msg, ButtplugClientMessageV4::RequestServerInfo(_)) {
        Some(message::ErrorV0::from(ButtplugError::from(
          ButtplugHandshakeError::RequestServerInfoExpected,
        )))
      } else {
        None
      };
      if let Some(mut return_error) = error {
        return_error.set_id(msg.id());
        return future::ready(Err(return_error)).boxed();
      }
      // If we haven't pinged out and we got an RSI message, fall thru.
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
        ButtplugClientMessageV4::RequestServerInfo(rsi_msg) => self.perform_handshake(rsi_msg),
        ButtplugClientMessageV4::Ping(p) => self.handle_ping(p),
        _ => ButtplugMessageError::UnexpectedMessageType(format!("{:?}", msg)).into(),
      }
    };
    // Simple way to set the ID on the way out. Just rewrap
    // the returned future to make sure it happens.
    async move {
      out_fut
        .await
        .map(|mut ok_msg| {
          ok_msg.set_id(id);
          ok_msg
        })
        .map_err(|err| {
          let mut error = message::ErrorV0::from(err);
          error.set_id(id);
          error
        })
    }
    .instrument(info_span!("Buttplug Server Message", id = id))
    .boxed()
  }

  /// Performs the [RequestServerInfo]([ServerInfo](crate::core::message::RequestServerInfo) /
  /// [ServerInfo](crate::core::message::ServerInfo) handshake, as specified in the [Buttplug
  /// Protocol Spec](https://buttplug-spec.docs.buttplug.io). This is the first thing that must
  /// happens upon connection to the server, in order to make sure the server can speak the same
  /// protocol version as the client.
  fn perform_handshake(&self, msg: message::RequestServerInfoV1) -> ButtplugServerResultFuture {
    if self.connected() {
      return ButtplugHandshakeError::HandshakeAlreadyHappened.into();
    }
    info!(
      "Performing server handshake check with client {} at message version {}.",
      msg.client_name(),
      msg.message_version()
    );

    #[cfg(not(feature = "allow-unstable-v4-connections"))]
    if BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION < msg.message_version() {
      return ButtplugHandshakeError::MessageSpecVersionMismatch(
        BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
        msg.message_version(),
      )
      .into();
    }
    // Only start the ping timer after we've received the handshake.
    let ping_timer = self.ping_timer.clone();
    let out_msg =
      message::ServerInfoV2::new(&self.server_name, msg.message_version(), self.max_ping_time);
    let connected = self.connected.clone();
    let mut name = self
      .client_name
      .try_write()
      .expect("We should never conflict on name access");
    *name = Some(msg.client_name().clone());
    async move {
      ping_timer.start_ping_timer().await;
      connected.store(true, Ordering::SeqCst);
      debug!("Server handshake check successful.");
      Result::Ok(out_msg.into())
    }
    .boxed()
  }

  /// Update the [PingTimer] with the latest received ping message.
  fn handle_ping(&self, msg: message::PingV0) -> ButtplugServerResultFuture {
    if self.max_ping_time == 0 {
      return ButtplugPingError::PingTimerNotRunning.into();
    }
    let fut = self.ping_timer.update_ping_time();
    async move {
      fut.await;
      Result::Ok(message::OkV0::new(msg.id()).into())
    }
    .boxed()
  }
}

#[cfg(test)]
mod test {
  use crate::{
    core::message::{self, BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION},
    server::ButtplugServerBuilder,
  };
  #[tokio::test]
  async fn test_server_reuse() {
    let server = ButtplugServerBuilder::default().finish().unwrap();
    let msg =
      message::RequestServerInfoV1::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION);
    let mut reply = server.parse_message(msg.clone().into()).await;
    assert!(reply.is_ok(), "Should get back ok: {:?}", reply);

    reply = server.parse_message(msg.clone().into()).await;
    assert!(
      reply.is_err(),
      "Should get back err on double handshake: {:?}",
      reply
    );
    assert!(server.disconnect().await.is_ok(), "Should disconnect ok");

    reply = server.parse_message(msg.clone().into()).await;
    assert!(
      reply.is_ok(),
      "Should get back ok on handshake after disconnect: {:?}",
      reply
    );
  }
}
