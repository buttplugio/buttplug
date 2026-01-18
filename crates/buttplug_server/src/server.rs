// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server_message_conversion::ButtplugServerDeviceEventMessageConverter;

use super::{
  ButtplugServerResultFuture,
  device::ServerDeviceManager,
  message::{
    ButtplugClientMessageVariant,
    ButtplugServerMessageVariant,
    server_device_attributes::TryFromClientMessage,
    spec_enums::{
      ButtplugCheckedClientMessageV4,
      ButtplugDeviceCommandMessageUnionV4,
      ButtplugDeviceManagerMessageUnion,
    },
  },
  ping_timer::PingTimer,
  server_message_conversion::ButtplugServerMessageConverter,
};
use buttplug_core::{
  errors::*,
  message::{
    self,
    BUTTPLUG_CURRENT_API_MAJOR_VERSION,
    ButtplugMessage,
    ButtplugMessageSpecVersion,
    ButtplugServerMessageV4,
    ErrorV0,
    StopAllDevicesV4,
    StopScanningV0,
  },
  util::stream::convert_broadcast_receiver_to_stream,
};
use futures::{
  Stream,
  future::{self, BoxFuture, FutureExt},
};
use std::{
  fmt,
  sync::{
    Arc,
    RwLock,
  },
};
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tracing::info_span;
use tracing_futures::Instrument;

/// Connection state for the ButtplugServer.
/// Replaces separate connected/client_name/spec_version fields with explicit states.
#[derive(Debug, Clone)]
pub enum ConnectionState {
  /// Initial state, waiting for RequestServerInfo handshake
  AwaitingHandshake,
  /// Client connected and handshake completed
  Connected {
    client_name: String,
    spec_version: ButtplugMessageSpecVersion,
  },
  /// Client explicitly disconnected
  Disconnected,
  /// Connection lost due to ping timeout
  PingedOut,
}

impl Default for ConnectionState {
  fn default() -> Self {
    ConnectionState::AwaitingHandshake
  }
}

/// The server side of the Buttplug protocol. Frontend for connection to device management and
/// communication.
pub struct ButtplugServer {
  /// The name of the server, which is relayed to the client on connection (mostly for
  /// confirmation in UI dialogs)
  server_name: String,
  /// The maximum ping time, in milliseconds, for the server. If the server does not receive a
  /// [Ping](buttplug_core::messages::Ping) message in this amount of time after the handshake has
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
  /// Connection state - tracks handshake, client info, and disconnection reason.
  state: Arc<RwLock<ConnectionState>>,
  /// Broadcaster for server events. Receivers for this are handed out through the
  /// [ButtplugServer::event_stream()] method.
  output_sender: broadcast::Sender<ButtplugServerMessageV4>,
}

impl std::fmt::Debug for ButtplugServer {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ButtplugServer")
      .field("server_name", &self.server_name)
      .field("max_ping_time", &self.max_ping_time)
      .field("state", &self.state)
      .finish()
  }
}

impl ButtplugServer {
  pub(super) fn new(
    server_name: &str,
    max_ping_time: u32,
    ping_timer: Arc<PingTimer>,
    device_manager: Arc<ServerDeviceManager>,
    state: Arc<RwLock<ConnectionState>>,
    output_sender: broadcast::Sender<ButtplugServerMessageV4>,
  ) -> Self {
    ButtplugServer {
      server_name: server_name.to_owned(),
      max_ping_time,
      ping_timer,
      device_manager,
      state,
      output_sender,
    }
  }

  pub fn client_name(&self) -> Option<String> {
    let state = self.state.read().expect("State lock poisoned");
    match &*state {
      ConnectionState::Connected { client_name, .. } => Some(client_name.clone()),
      _ => None,
    }
  }

  pub fn spec_version(&self) -> Option<ButtplugMessageSpecVersion> {
    let state = self.state.read().expect("State lock poisoned");
    match &*state {
      ConnectionState::Connected { spec_version, .. } => Some(*spec_version),
      _ => None,
    }
  }

  /// Returns the current connection state.
  pub fn connection_state(&self) -> ConnectionState {
    self.state.read().expect("State lock poisoned").clone()
  }

  /// Retreive an async stream of ButtplugServerMessages. This is how the server sends out
  /// non-query-related updates to the system, including information on devices being added/removed,
  /// client disconnection, etc...
  pub fn event_stream(&self) -> impl Stream<Item = ButtplugServerMessageVariant> + use<> {
    let state = self.state.clone();
    let converter = ButtplugServerMessageConverter::new(None);
    let device_indexes: Vec<u32> = self
      .device_manager
      .devices()
      .iter()
      .map(|x| *x.key())
      .collect();
    let device_event_converter = ButtplugServerDeviceEventMessageConverter::new(device_indexes);
    self.server_version_event_stream().map(move |m| {
      // Get spec_version from Connected state, default to Version4 if not connected
      let spec_version = {
        let state_guard = state.read().expect("State lock poisoned");
        match &*state_guard {
          ConnectionState::Connected { spec_version, .. } => *spec_version,
          _ => ButtplugMessageSpecVersion::Version4,
        }
      };
      if let ButtplugServerMessageV4::DeviceList(list) = m {
        device_event_converter.convert_device_list(&spec_version, &list)
      } else {
        // If we get an event and don't have a spec version yet, just throw out the latest.
        converter.convert_outgoing(&m, &spec_version).unwrap()
      }
    })
  }

  /// Retreive an async stream of ButtplugServerMessages, always at the latest available message
  /// spec. This is how the server sends out non-query-related updates to the system, including
  /// information on devices being added/removed, client disconnection, etc...
  pub fn server_version_event_stream(&self) -> impl Stream<Item = ButtplugServerMessageV4> + use<> {
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
    matches!(
      *self.state.read().expect("State lock poisoned"),
      ConnectionState::Connected { .. }
    )
  }

  /// Disconnects the server from a client, if it is connected.
  pub fn disconnect(&self) -> BoxFuture<'_, Result<(), message::ErrorV0>> {
    debug!("Buttplug Server {} disconnect requested", self.server_name);
    let ping_timer = self.ping_timer.clone();
    // As long as StopScanning/StopAllDevices aren't changed across message specs, we can inject
    // them using parse_checked_message and bypass version checking.
    let stop_scanning_fut = self.parse_checked_message(
      ButtplugCheckedClientMessageV4::StopScanning(StopScanningV0::default()),
    );
    let stop_fut = self.parse_checked_message(ButtplugCheckedClientMessageV4::StopAllDevices(
      StopAllDevicesV4::default(),
    ));
    let state = self.state.clone();
    async move {
      {
        let mut state_guard = state.write().expect("State lock poisoned");
        *state_guard = ConnectionState::Disconnected;
      }
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

  /// Sends a [ButtplugClientMessage] to be parsed by the server (for handshake or ping), or passed
  /// into the server's [DeviceManager] for communication with devices.
  pub fn parse_message(
    &self,
    msg: ButtplugClientMessageVariant,
  ) -> BoxFuture<'static, Result<ButtplugServerMessageVariant, ButtplugServerMessageVariant>> {
    let features = self.device_manager().feature_map();
    let msg_id = msg.id();
    trace!("Server received: {:?}", msg);
    // Use stored spec_version from Connected state if available, otherwise derive from message
    let spec_version = self.spec_version().unwrap_or_else(|| msg.version());
    match msg {
      ButtplugClientMessageVariant::V4(msg) => {
        let internal_msg =
          match ButtplugCheckedClientMessageV4::try_from_client_message(msg, &features) {
            Ok(m) => m,
            Err(e) => {
              let mut err_msg = ErrorV0::from(e);
              err_msg.set_id(msg_id);
              return future::ready(Err(ButtplugServerMessageVariant::from(
                ButtplugServerMessageV4::from(err_msg),
              )))
              .boxed();
            }
          };
        let fut = self.parse_checked_message(internal_msg);
        async move {
          Ok(
            fut
              .await
              .map_err(|e| ButtplugServerMessageVariant::from(ButtplugServerMessageV4::from(e)))?
              .into(),
          )
        }
        .boxed()
      }
      msg => {
        let converter = ButtplugServerMessageConverter::new(Some(msg.clone()));
        match ButtplugCheckedClientMessageV4::try_from_client_message(msg, &features) {
          Ok(converted_msg) => {
            trace!("Converted message: {:?}", converted_msg);
            let fut = self.parse_checked_message(converted_msg);
            async move {
              let result = fut.await.map_err(|e| {
                converter
                  .convert_outgoing(&e.into(), &spec_version)
                  .unwrap()
              })?;
              let out_msg = converter
                .convert_outgoing(&result, &spec_version)
                .map_err(|e| {
                  converter
                    .convert_outgoing(
                      &ButtplugServerMessageV4::from(ErrorV0::from(e)),
                      &spec_version,
                    )
                    .unwrap()
                });
              trace!("Server returning: {:?}", out_msg);
              out_msg
            }
            .boxed()
          }
          Err(e) => {
            let mut err_msg = ErrorV0::from(e);
            err_msg.set_id(msg_id);

            future::ready(Err(
              converter
                .convert_outgoing(&ButtplugServerMessageV4::from(err_msg), &spec_version)
                .unwrap(),
            ))
            .boxed()
          }
        }
      }
    }
  }

  pub fn parse_checked_message(
    &self,
    msg: ButtplugCheckedClientMessageV4,
  ) -> BoxFuture<'static, Result<ButtplugServerMessageV4, message::ErrorV0>> {
    trace!(
      "Buttplug Server {} received message to client parse: {:?}",
      self.server_name, msg
    );
    let id = msg.id();

    // Check connection state for message validity
    {
      let state = self.state.read().expect("State lock poisoned");
      let error = match &*state {
        ConnectionState::PingedOut => {
          // Connection lost due to ping timeout
          Some(message::ErrorV0::from(ButtplugError::from(
            ButtplugPingError::PingedOut,
          )))
        }
        ConnectionState::Disconnected => {
          // Already disconnected, no reconnection allowed
          Some(message::ErrorV0::from(ButtplugError::from(
            ButtplugHandshakeError::ReconnectDenied,
          )))
        }
        ConnectionState::AwaitingHandshake => {
          // Only RSI messages allowed before handshake
          if !matches!(msg, ButtplugCheckedClientMessageV4::RequestServerInfo(_)) {
            Some(message::ErrorV0::from(ButtplugError::from(
              ButtplugHandshakeError::RequestServerInfoExpected,
            )))
          } else {
            None
          }
        }
        ConnectionState::Connected { .. } => {
          // Connected, all messages allowed
          None
        }
      };
      if let Some(mut return_error) = error {
        return_error.set_id(msg.id());
        return future::ready(Err(return_error)).boxed();
      }
    }
    // Produce whatever future is needed to reply to the message, this may be a
    // device command future, or something the server handles. All futures will
    // return Result<ButtplugServerMessage, ButtplugError>, and we'll handle
    // tagging the result with the message id in the future we put out as the
    // return value from this method.
    let out_fut = if ButtplugDeviceManagerMessageUnion::try_from(msg.clone()).is_ok()
      || ButtplugDeviceCommandMessageUnionV4::try_from(msg.clone()).is_ok()
    {
      self.device_manager.parse_message(msg.clone())
    } else {
      match msg {
        ButtplugCheckedClientMessageV4::RequestServerInfo(rsi_msg) => {
          self.perform_handshake(rsi_msg)
        }
        ButtplugCheckedClientMessageV4::Ping(p) => self.handle_ping(p),
        _ => ButtplugMessageError::UnexpectedMessageType(format!("{msg:?}")).into(),
      }
    };
    // Simple way to set the ID on the way out. Just rewrap
    // the returned future to make sure it happens.
    async move {
      out_fut
        .await
        .map(|mut ok_msg| {
          ok_msg.set_id(id);
          trace!("Server returning message: {:?}", ok_msg);
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

  /// Performs the [RequestServerInfo]([ServerInfo](buttplug_core::message::RequestServerInfo) /
  /// [ServerInfo](buttplug_core::message::ServerInfo) handshake, as specified in the [Buttplug
  /// Protocol Spec](https://buttplug-spec.docs.buttplug.io). This is the first thing that must
  /// happens upon connection to the server, in order to make sure the server can speak the same
  /// protocol version as the client.
  fn perform_handshake(&self, msg: message::RequestServerInfoV4) -> ButtplugServerResultFuture {
    // Check current state for handshake validity
    {
      let state = self.state.read().expect("State lock poisoned");
      match &*state {
        ConnectionState::Connected { .. } => {
          return ButtplugHandshakeError::HandshakeAlreadyHappened.into();
        }
        ConnectionState::Disconnected | ConnectionState::PingedOut => {
          return ButtplugHandshakeError::ReconnectDenied.into();
        }
        ConnectionState::AwaitingHandshake => {
          // This is the expected state, continue with handshake
        }
      }
    }

    info!(
      "Performing server handshake check with client {} at message version {}.{}",
      msg.client_name(),
      msg.protocol_version_major(),
      msg.protocol_version_minor()
    );

    if BUTTPLUG_CURRENT_API_MAJOR_VERSION < msg.protocol_version_major() {
      return ButtplugHandshakeError::MessageSpecVersionMismatch(
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        msg.protocol_version_major(),
      )
      .into();
    }

    // Only start the ping timer after we've received the handshake.
    let ping_timer = self.ping_timer.clone();

    // Due to programming/spec errors in prior versions of the protocol, anything before v4 expected
    // that it would be back a matching api version of the server. The correct response is to send back whatever the
    let output_version = if (msg.protocol_version_major() as u32) < 4 {
      msg.protocol_version_major()
    } else {
      BUTTPLUG_CURRENT_API_MAJOR_VERSION
    };
    let out_msg =
      message::ServerInfoV4::new(&self.server_name, output_version, 0, self.max_ping_time);

    // protocol_version_major() returns ButtplugMessageSpecVersion directly
    let spec_version = msg.protocol_version_major();
    let client_name = msg.client_name().to_owned();
    let state = self.state.clone();

    async move {
      ping_timer.start_ping_timer().await;
      {
        let mut state_guard = state.write().expect("State lock poisoned");
        *state_guard = ConnectionState::Connected {
          client_name,
          spec_version,
        };
      }
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
  use crate::ButtplugServerBuilder;
  use buttplug_core::message::{self, BUTTPLUG_CURRENT_API_MAJOR_VERSION};
  #[tokio::test]
  async fn test_server_deny_reuse() {
    let server = ButtplugServerBuilder::default().finish().unwrap();
    let msg =
      message::RequestServerInfoV4::new("Test Client", BUTTPLUG_CURRENT_API_MAJOR_VERSION, 0);
    let mut reply = server.parse_checked_message(msg.clone().into()).await;
    assert!(reply.is_ok(), "Should get back ok: {:?}", reply);

    reply = server.parse_checked_message(msg.clone().into()).await;
    assert!(
      reply.is_err(),
      "Should get back err on double handshake: {:?}",
      reply
    );
    assert!(server.disconnect().await.is_ok(), "Should disconnect ok");

    reply = server.parse_checked_message(msg.clone().into()).await;
    assert!(
      reply.is_err(),
      "Should get back err on handshake after disconnect: {:?}",
      reply
    );
  }
}
