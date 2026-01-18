// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{
  ButtplugServerError,
  device::{ServerDeviceManager, ServerDeviceManagerBuilder},
  ping_timer::PingTimer,
  server::{ButtplugServer, ConnectionState},
};
use buttplug_core::{
  errors::*,
  message::{self, ButtplugServerMessageV4, StopAllDevicesV4},
  util::async_manager,
};
use buttplug_server_device_config::DeviceConfigurationManagerBuilder;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

/// Configures and creates [ButtplugServer] instances.
pub struct ButtplugServerBuilder {
  /// Name of the server, will be sent to the client as part of the [initial connection
  /// handshake](https://buttplug-spec.docs.buttplug.io/architecture.html#stages).
  name: String,
  /// Maximum time system will live without receiving a Ping message before disconnecting. If None,
  /// ping timer does not run.
  max_ping_time: Option<u32>,
  /// Device manager builder for the server
  device_manager: Arc<ServerDeviceManager>,
}

impl Default for ButtplugServerBuilder {
  fn default() -> Self {
    Self {
      name: "Buttplug Server".to_owned(),
      max_ping_time: None,
      device_manager: Arc::new(
        ServerDeviceManagerBuilder::new(
          DeviceConfigurationManagerBuilder::default()
            .finish()
            .unwrap(),
        )
        .finish()
        .unwrap(),
      ),
    }
  }
}

impl ButtplugServerBuilder {
  pub fn new(device_manager: ServerDeviceManager) -> Self {
    Self {
      name: "Buttplug Server".to_owned(),
      max_ping_time: None,
      device_manager: Arc::new(device_manager),
    }
  }

  pub fn with_shared_device_manager(device_manager: Arc<ServerDeviceManager>) -> Self {
    Self {
      name: "Buttplug Server".to_owned(),
      max_ping_time: None,
      device_manager,
    }
  }

  /// Set the name of the server, which is relayed to the client on connection (mostly for
  /// confirmation in UI dialogs)
  pub fn name(&mut self, name: &str) -> &mut Self {
    self.name = name.to_owned();
    self
  }

  /// Set the maximum ping time, in milliseconds, for the server. If the server does not receive a
  /// [Ping](buttplug_core::messages::Ping) message in this amount of time after the handshake has
  /// succeeded, the server will automatically disconnect. If this is not called, the ping timer
  /// will not be activated.
  ///
  /// Note that this has nothing to do with communication medium specific pings, like those built
  /// into the Websocket protocol. This ping is specific to the Buttplug protocol.
  pub fn max_ping_time(&mut self, ping_time: u32) -> &mut Self {
    self.max_ping_time = Some(ping_time);
    self
  }

  /// Try to build a [ButtplugServer] using the parameters given.
  pub fn finish(&self) -> Result<ButtplugServer, ButtplugServerError> {
    // Create the server
    debug!("Creating server '{}'", self.name);

    // Set up our channels to different parts of the system.
    let (output_sender, _) = broadcast::channel(256);

    // Connection state - starts in AwaitingHandshake
    let state = Arc::new(RwLock::new(ConnectionState::default()));

    let ping_time = self.max_ping_time.unwrap_or(0);

    // Create the ping timeout callback if ping time is configured.
    // The callback handles: updating state, stopping devices, and sending error.
    let ping_timeout_callback = if ping_time > 0 {
      let state_clone = state.clone();
      let device_manager_clone = self.device_manager.clone();
      let output_sender_clone = output_sender.clone();

      Some(move || {
        error!("Ping out signal received, stopping server");
        // Update connection state to PingedOut
        {
          let mut state_guard = state_clone.write().expect("State lock poisoned");
          *state_guard = ConnectionState::PingedOut;
        }
        // Stop all devices (spawn async task since callback is sync)
        async_manager::spawn(async move {
          if let Err(e) = device_manager_clone
            .stop_all_devices(&StopAllDevicesV4::default())
            .await
          {
            error!("Could not stop devices on ping timeout: {:?}", e);
          }
        });
        // Send error to output channel
        if output_sender_clone
          .send(ButtplugServerMessageV4::Error(message::ErrorV0::from(
            ButtplugError::from(ButtplugPingError::PingedOut),
          )))
          .is_err()
        {
          error!("Server disappeared, cannot update about ping out.");
        };
      })
    } else {
      None
    };

    let ping_timer = Arc::new(PingTimer::new(ping_time, ping_timeout_callback));

    // Assuming everything passed, return the server.
    Ok(ButtplugServer::new(
      &self.name,
      ping_time,
      ping_timer,
      self.device_manager.clone(),
      state,
      output_sender,
    ))
  }
}
