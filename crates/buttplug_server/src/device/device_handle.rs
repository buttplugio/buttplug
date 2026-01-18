// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Device Handle - Lightweight reference to a device task
//!
//! The DeviceHandle provides a lightweight way to interact with a device. It contains:
//! - A channel sender to send commands to the device's unified task
//! - Cached immutable metadata (name, features, etc.) that doesn't require task communication
//!
//! This replaces the previous Arc<ServerDevice> pattern, reducing Arc cloning and clarifying
//! ownership: the device task owns the actual hardware and protocol state, while handles
//! provide a way to send commands and query cached metadata.

use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugError},
  message::{ButtplugServerMessageV4, DeviceFeature},
};
use buttplug_server_device_config::{ServerDeviceDefinition, UserDeviceIdentifier};
use getset::{CopyGetters, Getters};
use tokio::sync::{mpsc, oneshot};

use crate::message::{checked_input_cmd::CheckedInputCmdV4, checked_output_cmd::CheckedOutputCmdV4};

/// Commands that can be sent to a device's unified task
#[derive(Debug)]
pub enum DeviceCommand {
  /// Output command (vibrate, rotate, linear, etc.)
  Output {
    cmd: CheckedOutputCmdV4,
    response: oneshot::Sender<Result<(), ButtplugError>>,
  },
  /// Input command (read sensor, subscribe, unsubscribe)
  /// Returns ButtplugServerMessageV4 which can be InputReadingV4 (for reads) or OkV0 (for subscribe/unsubscribe)
  Input {
    cmd: CheckedInputCmdV4,
    response: oneshot::Sender<Result<ButtplugServerMessageV4, ButtplugError>>,
  },
  /// Stop all device outputs
  Stop {
    response: oneshot::Sender<Result<(), ButtplugError>>,
  },
  /// Disconnect the device gracefully
  Disconnect,
}

/// Lightweight handle to a device
///
/// This struct is cheap to clone (just a channel sender + cached metadata).
/// The actual device state lives in the device task; this handle provides
/// a way to send commands and access immutable metadata.
#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct DeviceHandle {
  /// Channel to send commands to the device task
  command_tx: mpsc::Sender<DeviceCommand>,

  /// Cached immutable device attributes
  #[getset(get = "pub")]
  identifier: UserDeviceIdentifier,

  #[getset(get_copy = "pub")]
  index: u32,

  #[getset(get = "pub")]
  name: String,

  #[getset(get = "pub")]
  display_name: Option<String>,

  #[getset(get = "pub")]
  features: Vec<DeviceFeature>,
}

impl DeviceHandle {
  /// Create a new device handle
  ///
  /// This should only be called by the device creation code, not by users.
  pub(crate) fn new(
    command_tx: mpsc::Sender<DeviceCommand>,
    identifier: UserDeviceIdentifier,
    definition: &ServerDeviceDefinition,
  ) -> Result<Self, ButtplugError> {
    // Cache the device features
    let features: Vec<_> = definition
      .features()
      .values()
      .filter_map(|f| f.as_device_feature().ok())
      .collect();

    Ok(Self {
      command_tx,
      identifier,
      index: definition.index(),
      name: definition.name().to_string(),
      display_name: definition.display_name().clone(),
      features,
    })
  }

  /// Send an output command to the device (vibrate, rotate, etc.)
  pub async fn send_output(&self, cmd: CheckedOutputCmdV4) -> Result<(), ButtplugError> {
    let (tx, rx) = oneshot::channel();
    self
      .command_tx
      .send(DeviceCommand::Output { cmd, response: tx })
      .await
      .map_err(|_| ButtplugDeviceError::DeviceNotConnected("channel closed".to_string()))?;
    rx.await
      .map_err(|_| ButtplugDeviceError::DeviceNotConnected("channel closed".to_string()))?
  }

  /// Send an input command to the device (read sensor, subscribe, etc.)
  ///
  /// Returns ButtplugServerMessageV4 which can be:
  /// - InputReadingV4 for read commands
  /// - OkV0 for subscribe/unsubscribe commands
  pub async fn send_input(
    &self,
    cmd: CheckedInputCmdV4,
  ) -> Result<ButtplugServerMessageV4, ButtplugError> {
    let (tx, rx) = oneshot::channel();
    self
      .command_tx
      .send(DeviceCommand::Input { cmd, response: tx })
      .await
      .map_err(|_| ButtplugDeviceError::DeviceNotConnected("channel closed".to_string()))?;
    rx.await
      .map_err(|_| ButtplugDeviceError::DeviceNotConnected("channel closed".to_string()))?
  }

  /// Stop all device outputs
  pub async fn stop(&self) -> Result<(), ButtplugError> {
    let (tx, rx) = oneshot::channel();
    self
      .command_tx
      .send(DeviceCommand::Stop { response: tx })
      .await
      .map_err(|_| ButtplugDeviceError::DeviceNotConnected("channel closed".to_string()))?;
    rx.await
      .map_err(|_| ButtplugDeviceError::DeviceNotConnected("channel closed".to_string()))?
  }

  /// Request the device to disconnect
  ///
  /// This sends a disconnect command to the device task. The task will
  /// clean up and exit, which will trigger a Disconnected event.
  pub async fn disconnect(&self) -> Result<(), ButtplugError> {
    self
      .command_tx
      .send(DeviceCommand::Disconnect)
      .await
      .map_err(|_| ButtplugDeviceError::DeviceNotConnected("channel closed".to_string()).into())
  }

  /// Check if the device task is still running
  ///
  /// Returns false if the channel is closed (device task has exited).
  pub fn is_connected(&self) -> bool {
    !self.command_tx.is_closed()
  }
}
