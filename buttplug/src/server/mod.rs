// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handles client sessions, as well as discovery and communication with hardware.
//!
//! The Buttplug Server is just a thin frontend for device connection and communication. The server
//! itself doesn't do much other than configuring the device system and handling a few non-device
//! related tasks like [initial connection
//! handshake](https://buttplug-spec.docs.buttplug.io/architecture.html#stages) and system timeouts.
//! Once a connection is made from a [ButtplugClient](crate::client::ButtplugClient) to a
//! [ButtplugServer], the server mostly acts as a pass-thru frontend to the [DeviceManager].
//!
//! ## Server Lifetime
//!
//! The server has following lifetime stages:
//!
//! - Configuration
//!   - This happens across the [ButtplugServerBuilder], as well as the [ButtplugServer] instance it
//!     returns. During this time, we can specify attributes of the server like its name and if it
//!     will have a ping timer. It also allows for addition of protocols and device configurations
//!     to the system, either via configuration files or through manual API calls.
//! - Connection
//!   - After configuration is done, the server can be put into a listening mode (assuming
//!     [RemoteServer](ButtplugRemoteServer) is being used. for [in-process
//!     servers](crate::connector::ButtplugInProcessClientConnector), the client own the server and just
//!     connects to it directly). At this point, a [ButtplugClient](crate::client::ButtplugClient)
//!     can connect and start the
//!     [handshake](https://buttplug-spec.docs.buttplug.io/architecture.html#stages) process.
//! - Pass-thru
//!   - Once the handshake has succeeded, the server basically becomes a pass-thru to the
//!     [DeviceManager], which manages discovery of and communication with devices. The only thing
//!     the server instance manages at this point is ownership of the [DeviceManager] and
//!     ping timer, but doesn't really do much itself. The server remains in this state until the
//!     connection to the client is severed, at which point all devices connected to the device
//!     manager will be stopped.
//! - Disconnection
//!   - The server can be put back in Connection mode without being recreated after disconnection,
//!     to listen for another client connection while still maintaining connection to whatever
//!     devices the [DeviceManager] has.
//! - Destruction
//!   - If the server object is dropped, all devices are stopped and disconnected as part
//!     of the [DeviceManager] teardown.

pub mod device;
mod ping_timer;
mod remote_server;

pub use remote_server::ButtplugRemoteServer;

use crate::{
  core::{
    errors::*,
    messages::{
      self, ButtplugClientMessage, ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceManagerMessageUnion, ButtplugMessage, ButtplugServerMessage, StopAllDevices,
      StopScanning, BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
    },
  },
  util::{
    async_manager,
    device_configuration::{load_protocol_configs_from_json, DEVICE_CONFIGURATION_JSON},
    stream::convert_broadcast_receiver_to_stream,
  },
};
use device::manager::{DeviceManager, DeviceManagerBuilder};
use futures::{
  future::{self, BoxFuture},
  Stream,
};
use ping_timer::PingTimer;
use std::{
  fmt,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use thiserror::Error;
use tokio::sync::broadcast;
use tracing_futures::Instrument;

/// Result type for Buttplug Server methods, as the server will always communicate in
/// [ButtplugServerMessage] instances in order to follow the [Buttplug
/// Spec](http://buttplug-spec.docs.buttplug.io).
pub type ButtplugServerResult = Result<ButtplugServerMessage, ButtplugError>;
/// Future type for Buttplug Server futures, as the server will always communicate in
/// [ButtplugServerMessage] instances in order to follow the [Buttplug
/// Spec](http://buttplug-spec.docs.buttplug.io).
pub type ButtplugServerResultFuture = BoxFuture<'static, ButtplugServerResult>;

/// Error enum for Buttplug Server configuration errors.
#[derive(Error, Debug)]
pub enum ButtplugServerError {
  /// DeviceCommunicationManager type has already been added to the system.
  #[error("DeviceManager of type {0} has already been added.")]
  DeviceManagerTypeAlreadyAdded(String),
  /// Protocol has already been added to the system.
  #[error("Buttplug Protocol of type {0} has already been added to the system.")]
  ProtocolAlreadyAdded(String),
  /// Requested protocol has not been registered with the system.
  #[error("Buttplug Protocol of type {0} does not exist in the system and cannot be removed.")]
  ProtocolDoesNotExist(String),
}

/// Configures and creates [ButtplugServer] instances.
pub struct ButtplugServerBuilder {
  /// Name of the server, will be sent to the client as part of the [initial connection
  /// handshake](https://buttplug-spec.docs.buttplug.io/architecture.html#stages).
  name: String,
  /// Maximum time system will live without receiving a Ping message before disconnecting. If None,
  /// ping timer does not run.
  max_ping_time: Option<u32>,
  /// If true, allows sending/receiving of raw binary data from devices.
  allow_raw_messages: bool,
  /// JSON string, with the contents of the base Device Configuration file
  device_configuration_json: Option<String>,
  /// JSON string, with the contents of the User Device Configuration file
  user_device_configuration_json: Option<String>,
  /// Device manager builder for the server
  device_manager_builder: DeviceManagerBuilder
}

impl Default for ButtplugServerBuilder {
  fn default() -> Self {
    Self {
      name: "Buttplug Server".to_owned(),
      max_ping_time: None,
      allow_raw_messages: false,
      device_configuration_json: Some(DEVICE_CONFIGURATION_JSON.to_owned()),
      user_device_configuration_json: None,
      device_manager_builder: DeviceManagerBuilder::default()
    }
  }
}

impl ButtplugServerBuilder {
  /// Set the name of the server, which is relayed to the client on connection (mostly for
  /// confirmation in UI dialogs)
  pub fn name(&mut self, name: &str) -> &mut Self {
    self.name = name.to_owned();
    self
  }

  /// Set the maximum ping time, in milliseconds, for the server. If the server does not receive a
  /// [Ping](crate::core::messages::Ping) message in this amount of time after the handshake has
  /// succeeded, the server will automatically disconnect. If this is not called, the ping timer
  /// will not be activated.
  ///
  /// Note that this has nothing to do with communication medium specific pings, like those built
  /// into the Websocket protocol. This ping is specific to the Buttplug protocol.
  pub fn max_ping_time(&mut self, ping_time: u32) -> &mut Self {
    self.max_ping_time = Some(ping_time);
    self
  }

  /// If set to true, devices will be allowed to use Raw Messages.
  ///
  /// **Be careful with this.** Raw messages being allowed on devices can open up security issues
  /// like allowing for device firmware updates through library calls.
  pub fn allow_raw_messages(&mut self, allow: bool) -> &mut Self {
    self.allow_raw_messages = allow;
    self
  }

  /// Set the device configuration json file contents, to be loaded during build.
  pub fn device_configuration_json(&mut self, config_json: Option<String>) -> &mut Self {
    self.device_configuration_json = config_json;
    self
  }

  /// Set the user device configuration json file contents, to be loaded during build.
  pub fn user_device_configuration_json(&mut self, config_json: Option<String>) -> &mut Self {
    self.user_device_configuration_json = config_json;
    self
  }

  pub fn device_manager_builder(&mut self) -> &mut DeviceManagerBuilder {
    &mut self.device_manager_builder
  }

  /// Try to build a [ButtplugServer] using the parameters given.
  pub fn finish(&mut self) -> Result<ButtplugServer, ButtplugError> {
    // Create the server
    debug!("Creating server '{}'", self.name);
    info!("Buttplug Server Operating System Info: {}", os_info::get());

    // First, try loading our configs. If this doesn't work, nothing else will, so get it out of
    // the way first.
    let protocol_map = load_protocol_configs_from_json(
      self.device_configuration_json.clone(),
      self.user_device_configuration_json.clone(),
      false,
    )?;

    // Set up our channels to different parts of the system.
    let (output_sender, _) = broadcast::channel(256);
    let output_sender_clone = output_sender.clone();
    let connected = Arc::new(AtomicBool::new(false));
    let connected_clone = connected.clone();

    // Create the ping timer, since we'll need to pass it in to the DeviceManager on creation
    //
    // TODO this should use a cancellation token instead of passing around the timer itself.
    let ping_time = self.max_ping_time.unwrap_or(0);
    let ping_timer = Arc::new(PingTimer::new(ping_time));
    let ping_timeout_notifier = ping_timer.ping_timeout_waiter();

    // Spawn the ping timer task, assuming the ping time is > 0.
    if ping_time > 0 {
      async_manager::spawn(
        async move {
          // This will only exit if we've pinged out.
          ping_timeout_notifier.await;
          error!("Ping out signal received, stopping server");
          connected_clone.store(false, Ordering::SeqCst);
          // TODO Should the event sender return a result instead of an error message?
          if output_sender_clone
            .send(messages::Error::from(ButtplugError::from(ButtplugPingError::PingedOut)).into())
            .is_err()
          {
            error!("Server disappeared, cannot update about ping out.");
          };
        }
        .instrument(tracing::info_span!("Buttplug Server Ping Timeout Task")),
      );
    }

    if self.allow_raw_messages {
      self.device_manager_builder.allow_raw_messages();
    }

    for address in protocol_map.allow_list() {
      self.device_manager_builder.allowed_address(address);
    }

    for address in protocol_map.deny_list() {
      self.device_manager_builder.denied_address(address);
    }

    for (index, address) in protocol_map.reserved_indexes() {
      self.device_manager_builder.reserved_index(address, *index);
    }

    for (name, def) in protocol_map.protocol_configurations() {
      self.device_manager_builder.protocol_device_configuration(name, def);
    }

    let device_manager = self.device_manager_builder.finish(output_sender.clone())?;

    // Assuming everything passed, return the server.
    Ok(ButtplugServer {
      server_name: self.name.clone(),
      max_ping_time: ping_time,
      device_manager,
      ping_timer,
      connected,
      output_sender,
    })
  }
}

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
  device_manager: DeviceManager,
  /// If true, client is currently connected to server
  connected: Arc<AtomicBool>,
  /// Broadcaster for server events. Receivers for this are handed out through the
  /// [ButtplugServer::event_stream()] method.
  output_sender: broadcast::Sender<ButtplugServerMessage>,
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

impl Default for ButtplugServer {
  /// Creates a default Buttplug Server, with no ping time, and no raw message support.
  fn default() -> Self {
    // We can unwrap here because if default init fails, so will pretty much every test.
    ButtplugServerBuilder::default()
      .finish()
      .expect("Default is infallible")
  }
}

impl ButtplugServer {
  /// Retreive an async stream of ButtplugServerMessages. This is how the server sends out
  /// non-query-related updates to the system, including information on devices being added/removed,
  /// client disconnection, etc...
  pub fn event_stream(&self) -> impl Stream<Item = ButtplugServerMessage> {
    // Unlike the client API, we can expect anyone using the server to pin this
    // themselves.
    convert_broadcast_receiver_to_stream(self.output_sender.subscribe())
  }

  /// Returns a references to the internal device manager, for handling configuration.
  pub fn device_manager(&self) -> &DeviceManager {
    &self.device_manager
  }

  /// If true, client is currently connected to the server.
  pub fn connected(&self) -> bool {
    self.connected.load(Ordering::SeqCst)
  }

  /// Disconnects the server from a client, if it is connected.
  pub fn disconnect(&self) -> BoxFuture<Result<(), messages::Error>> {
    debug!("Buttplug Server {} disconnect requested", self.server_name);
    let ping_timer = self.ping_timer.clone();
    let stop_scanning_fut =
      self.parse_message(ButtplugClientMessage::StopScanning(StopScanning::default()));
    let stop_fut = self.parse_message(ButtplugClientMessage::StopAllDevices(
      StopAllDevices::default(),
    ));
    let connected = self.connected.clone();
    Box::pin(async move {
      connected.store(false, Ordering::SeqCst);
      ping_timer.stop_ping_timer().await;
      // Ignore returns here, we just want to stop.
      info!("Server disconnected, stopping device scanning if it was started...");
      let _ = stop_scanning_fut.await;
      info!("Server disconnected, stopping all devices...");
      let _ = stop_fut.await;
      Ok(())
    })
  }


  /// Sends a [ButtplugClientMessage] to be parsed by the server (for handshake or ping), or passed
  /// into the server's [DeviceManager] for communication with devices.
  pub fn parse_message(
    &self,
    msg: ButtplugClientMessage,
  ) -> BoxFuture<'static, Result<ButtplugServerMessage, messages::Error>> {
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
        Some(messages::Error::from(ButtplugError::from(
          ButtplugPingError::PingedOut,
        )))
      } else if !matches!(msg, ButtplugClientMessage::RequestServerInfo(_)) {
        Some(messages::Error::from(ButtplugError::from(
          ButtplugHandshakeError::RequestServerInfoExpected,
        )))
      } else {
        None
      };
      if let Some(mut return_error) = error {
        return_error.set_id(msg.id());
        return Box::pin(future::ready(Err(return_error)));
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
        ButtplugClientMessage::RequestServerInfo(rsi_msg) => self.perform_handshake(rsi_msg),
        ButtplugClientMessage::Ping(p) => self.handle_ping(p),
        _ => ButtplugMessageError::UnexpectedMessageType(format!("{:?}", msg)).into(),
      }
    };
    // Simple way to set the ID on the way out. Just rewrap
    // the returned future to make sure it happens.
    Box::pin(
      async move {
        out_fut
          .await
          .map(|mut ok_msg| {
            ok_msg.set_id(id);
            ok_msg
          })
          .map_err(|err| {
            let mut error = messages::Error::from(err);
            error.set_id(id);
            error
          })
      }
      .instrument(info_span!("Buttplug Server Message", id = id)),
    )
  }

  /// Performs the [RequestServerInfo]([ServerInfo](crate::core::message::RequestServerInfo) /
  /// [ServerInfo](crate::core::message::ServerInfo) handshake, as specified in the [Buttplug
  /// Protocol Spec](https://buttplug-spec.docs.buttplug.io). This is the first thing that must
  /// happens upon connection to the server, in order to make sure the server can speak the same
  /// protocol version as the client.
  fn perform_handshake(&self, msg: messages::RequestServerInfo) -> ButtplugServerResultFuture {
    if self.connected() {
      return ButtplugHandshakeError::HandshakeAlreadyHappened.into();
    }
    info!(
      "Performing server handshake check with client {} at message version {}.",
      msg.client_name(),
      msg.message_version()
    );
    if BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION < msg.message_version() {
      return ButtplugHandshakeError::MessageSpecVersionMismatch(
        BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
        msg.message_version(),
      )
      .into();
    }
    // Only start the ping timer after we've received the handshake.
    let ping_timer = self.ping_timer.clone();
    let out_msg = messages::ServerInfo::new(
      &self.server_name,
      BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
      self.max_ping_time,
    );
    let connected = self.connected.clone();
    Box::pin(async move {
      ping_timer.start_ping_timer().await;
      connected.store(true, Ordering::SeqCst);
      debug!("Server handshake check successful.");
      Result::Ok(out_msg.into())
    })
  }

  /// Update the [PingTimer] with the latest received ping message.
  fn handle_ping(&self, msg: messages::Ping) -> ButtplugServerResultFuture {
    if self.max_ping_time == 0 {
      return ButtplugPingError::PingTimerNotRunning.into();
    }
    let fut = self.ping_timer.update_ping_time();
    Box::pin(async move {
      fut.await;
      Result::Ok(messages::Ok::new(msg.id()).into())
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
    async_manager::block_on(async {
      let server = ButtplugServer::default();
      let msg =
        messages::RequestServerInfo::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION);
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
    });
  }
}
