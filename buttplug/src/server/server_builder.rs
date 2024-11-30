// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{
  device::{
    configuration::DeviceConfigurationManagerBuilder,
    ServerDeviceManager,
    ServerDeviceManagerBuilder,
  },
  ping_timer::PingTimer,
  server::ButtplugServer,
  ButtplugServerError,
};
use crate::{
  core::{
    errors::*,
    message::{self, ButtplugServerMessageV4},
  },
  util::async_manager,
};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};
use tokio::sync::broadcast;
use tracing_futures::Instrument;

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
  /// Allow connections for clients using beta v4 message spec support (message spec may change and break for now)
  allow_v4_connections: bool
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
      #[cfg(not(feature = "default_v4_spec"))]
      allow_v4_connections: false,
      #[cfg(feature = "default_v4_spec")]
      allow_v4_connections: true,
    }
  }
}

impl ButtplugServerBuilder {
  pub fn new(device_manager: ServerDeviceManager) -> Self {
    Self {
      name: "Buttplug Server".to_owned(),
      max_ping_time: None,
      device_manager: Arc::new(device_manager),
      #[cfg(not(feature = "default_v4_spec"))]
      allow_v4_connections: false,
      #[cfg(feature = "default_v4_spec")]
      allow_v4_connections: true,
    }
  }

  pub fn with_shared_device_manager(device_manager: Arc<ServerDeviceManager>) -> Self {
    Self {
      name: "Buttplug Server".to_owned(),
      max_ping_time: None,
      device_manager,
      allow_v4_connections: false
    }
  }

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

  pub fn allow_v4_connections(&mut self) -> &mut Self {
    self.allow_v4_connections = true;
    self
  }

  /// Try to build a [ButtplugServer] using the parameters given.
  pub fn finish(&self) -> Result<ButtplugServer, ButtplugServerError> {
    // Create the server
    debug!("Creating server '{}'", self.name);
    info!("Buttplug Server Operating System Info: {}", os_info::get());

    // Set up our channels to different parts of the system.
    let (output_sender, _) = broadcast::channel(256);
    let output_sender_clone = output_sender.clone();

    let connected = Arc::new(AtomicBool::new(false));
    let connected_clone = connected.clone();

    // TODO this should use a cancellation token instead of passing around the timer itself.
    let ping_time = self.max_ping_time.unwrap_or(0);
    let ping_timer = Arc::new(PingTimer::new(ping_time));
    let ping_timeout_notifier = ping_timer.ping_timeout_waiter();

    // Spawn the ping timer task, assuming the ping time is > 0.
    if ping_time > 0 {
      let device_manager_clone = self.device_manager.clone();
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
            .send(ButtplugServerMessageV4::Error(message::ErrorV0::from(
              ButtplugError::from(ButtplugPingError::PingedOut),
            )))
            .is_err()
          {
            error!("Server disappeared, cannot update about ping out.");
          };
        }
        .instrument(tracing::info_span!("Buttplug Server Ping Timeout Task")),
      );
    }

    if self.allow_v4_connections {
      warn!("Allowing beta v4 connections. Note that things may break due to message spec changes.");
    }

    // Assuming everything passed, return the server.
    Ok(ButtplugServer::new(
      &self.name,
      ping_time,
      ping_timer,
      self.device_manager.clone(),
      connected,
      output_sender,
      self.allow_v4_connections
    ))
  }
}
