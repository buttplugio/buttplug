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

use self::device::{
  configuration::{
    ProtocolAttributesIdentifier,
    ProtocolCommunicationSpecifier,
    ProtocolDeviceAttributes,
  },
  hardware::communication::HardwareCommunicationManagerBuilder,
  protocol::ProtocolIdentifierFactory,
  ServerDeviceIdentifier,
  ServerDeviceManager,
  ServerDeviceManagerBuilder,
};
use crate::{
  core::{
    errors::*,
    message::{
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
  },
  util::{
    async_manager,
    device_configuration::{load_protocol_configs, DEVICE_CONFIGURATION_JSON},
    stream::convert_broadcast_receiver_to_stream,
  },
};
use futures::{
  future::{self, BoxFuture, FutureExt},
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
use tokio_stream::StreamExt;
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
  /// DeviceConfigurationManager could not be built.
  #[error("The DeviceConfigurationManager could not be built: {0}")]
  DeviceConfigurationManagerError(ButtplugDeviceError),
  /// DeviceCommunicationManager type has already been added to the system.
  #[error("DeviceCommunicationManager of type {0} has already been added.")]
  DeviceCommunicationManagerTypeAlreadyAdded(String),
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
  /// JSON string, with the contents of the base Device Configuration file
  device_configuration_json: Option<String>,
  /// JSON string, with the contents of the User Device Configuration file
  user_device_configuration_json: Option<String>,
  /// Device manager builder for the server
  device_manager_builder: ServerDeviceManagerBuilder,
}

impl Default for ButtplugServerBuilder {
  fn default() -> Self {
    Self {
      name: "Buttplug Server".to_owned(),
      max_ping_time: None,
      device_configuration_json: Some(DEVICE_CONFIGURATION_JSON.to_owned()),
      user_device_configuration_json: None,
      device_manager_builder: ServerDeviceManagerBuilder::default(),
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

  pub fn comm_manager<T>(&mut self, builder: T) -> &mut Self
  where
    T: HardwareCommunicationManagerBuilder + 'static,
  {
    self.device_manager_builder.comm_manager(builder);
    self
  }

  pub fn allowed_address(&mut self, address: &str) -> &mut Self {
    self.device_manager_builder.allowed_address(address);
    self
  }

  pub fn denied_address(&mut self, address: &str) -> &mut Self {
    self.device_manager_builder.denied_address(address);
    self
  }

  pub fn reserved_index(&mut self, identifier: &ServerDeviceIdentifier, index: u32) -> &mut Self {
    self
      .device_manager_builder
      .reserved_index(identifier, index);
    self
  }

  pub fn protocol_factory<T>(&mut self, factory: T) -> &mut Self
  where
    T: ProtocolIdentifierFactory + 'static,
  {
    self.device_manager_builder.protocol_factory(factory);
    self
  }

  pub fn communication_specifier(
    &mut self,
    protocol_name: &str,
    specifier: ProtocolCommunicationSpecifier,
  ) -> &mut Self {
    self
      .device_manager_builder
      .communication_specifier(protocol_name, specifier);
    self
  }

  pub fn protocol_attributes(
    &mut self,
    identifier: ProtocolAttributesIdentifier,
    attributes: ProtocolDeviceAttributes,
  ) -> &mut Self {
    self
      .device_manager_builder
      .protocol_attributes(identifier, attributes);
    self
  }

  pub fn skip_default_protocols(&mut self) -> &mut Self {
    self.device_manager_builder.skip_default_protocols();
    self
  }

  pub fn allow_raw_messages(&mut self) -> &mut Self {
    self.device_manager_builder.allow_raw_messages();
    self
  }

  /// Try to build a [ButtplugServer] using the parameters given.
  pub fn finish(&mut self) -> Result<ButtplugServer, ButtplugServerError> {
    // Create the server
    debug!("Creating server '{}'", self.name);
    info!("Buttplug Server Operating System Info: {}", os_info::get());

    // First, try loading our configs. If this doesn't work, nothing else will, so get it out of
    // the way first.
    let dcm_builder = load_protocol_configs(
      self.device_configuration_json.clone(),
      self.user_device_configuration_json.clone(),
      false,
    )
    .map_err(ButtplugServerError::DeviceConfigurationManagerError)?;

    self
      .device_manager_builder
      .device_configuration_manager_builder(&dcm_builder);
    // Set up our channels to different parts of the system.
    let (output_sender, _) = broadcast::channel(256);
    let output_sender_clone = output_sender.clone();

    let device_manager = Arc::new(self.device_manager_builder.finish()?);

    let connected = Arc::new(AtomicBool::new(false));
    let connected_clone = connected.clone();

    // TODO this should use a cancellation token instead of passing around the timer itself.
    let ping_time = self.max_ping_time.unwrap_or(0);
    let ping_timer = Arc::new(PingTimer::new(ping_time));
    let ping_timeout_notifier = ping_timer.ping_timeout_waiter();

    // Spawn the ping timer task, assuming the ping time is > 0.
    if ping_time > 0 {
      let device_manager_clone = device_manager.clone();
      async_manager::spawn(
        async move {
          // This will only exit if we've pinged out.
          ping_timeout_notifier.await;
          error!("Ping out signal received, stopping server");
          connected_clone.store(false, Ordering::SeqCst);
          async_manager::spawn(async move {
            if let Err(e) = device_manager_clone.stop_all_devices().await {
              error!("Could not stop devices on ping timeout: {:?}", e);
            }
          });
          // TODO Should the event sender return a result instead of an error message?
          if output_sender_clone
            .send(message::Error::from(ButtplugError::from(ButtplugPingError::PingedOut)).into())
            .is_err()
          {
            error!("Server disappeared, cannot update about ping out.");
          };
        }
        .instrument(tracing::info_span!("Buttplug Server Ping Timeout Task")),
      );
    }

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
  device_manager: Arc<ServerDeviceManager>,
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
  pub fn disconnect(&self) -> BoxFuture<Result<(), message::Error>> {
    debug!("Buttplug Server {} disconnect requested", self.server_name);
    let ping_timer = self.ping_timer.clone();
    let stop_scanning_fut =
      self.parse_message(ButtplugClientMessage::StopScanning(StopScanning::default()));
    let stop_fut = self.parse_message(ButtplugClientMessage::StopAllDevices(
      StopAllDevices::default(),
    ));
    let connected = self.connected.clone();
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

  /// Sends a [ButtplugClientMessage] to be parsed by the server (for handshake or ping), or passed
  /// into the server's [DeviceManager] for communication with devices.
  pub fn parse_message(
    &self,
    msg: ButtplugClientMessage,
  ) -> BoxFuture<'static, Result<ButtplugServerMessage, message::Error>> {
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
        Some(message::Error::from(ButtplugError::from(
          ButtplugPingError::PingedOut,
        )))
      } else if !matches!(msg, ButtplugClientMessage::RequestServerInfo(_)) {
        Some(message::Error::from(ButtplugError::from(
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
        ButtplugClientMessage::RequestServerInfo(rsi_msg) => self.perform_handshake(rsi_msg),
        ButtplugClientMessage::Ping(p) => self.handle_ping(p),
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
          let mut error = message::Error::from(err);
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
  fn perform_handshake(&self, msg: message::RequestServerInfo) -> ButtplugServerResultFuture {
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
    let out_msg =
      message::ServerInfo::new(&self.server_name, msg.message_version(), self.max_ping_time);
    let connected = self.connected.clone();
    async move {
      ping_timer.start_ping_timer().await;
      connected.store(true, Ordering::SeqCst);
      debug!("Server handshake check successful.");
      Result::Ok(out_msg.into())
    }
    .boxed()
  }

  /// Update the [PingTimer] with the latest received ping message.
  fn handle_ping(&self, msg: message::Ping) -> ButtplugServerResultFuture {
    if self.max_ping_time == 0 {
      return ButtplugPingError::PingTimerNotRunning.into();
    }
    let fut = self.ping_timer.update_ping_time();
    async move {
      fut.await;
      Result::Ok(message::Ok::new(msg.id()).into())
    }
    .boxed()
  }

  pub fn shutdown(&self) -> ButtplugServerResultFuture {
    let device_manager = self.device_manager.clone();
    //let disconnect_future = self.disconnect();
    async move { device_manager.shutdown().await }.boxed()
  }
}

#[cfg(test)]
mod test {
  use crate::{
    core::message::{self, BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION},
    server::ButtplugServer,
  };

  #[tokio::test]
  async fn test_server_reuse() {
    let server = ButtplugServer::default();
    let msg = message::RequestServerInfo::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION);
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
