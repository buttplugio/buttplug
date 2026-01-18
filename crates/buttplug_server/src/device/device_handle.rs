// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Device Handle - Lightweight facade for device communication
//!
//! DeviceHandle provides a simplified interface for sending commands to devices.
//! It wraps the underlying device implementation and provides a unified command
//! interface through the DeviceCommand enum.

use std::sync::Arc;

use buttplug_core::{ButtplugResultFuture, errors::ButtplugError, message::DeviceMessageInfoV4};
use buttplug_server_device_config::{ServerDeviceDefinition, UserDeviceIdentifier};
use tokio::sync::oneshot;

use crate::{
  ButtplugServerResultFuture,
  message::{
    checked_input_cmd::CheckedInputCmdV4,
    checked_output_cmd::CheckedOutputCmdV4,
    server_device_attributes::ServerDeviceAttributes,
    spec_enums::ButtplugDeviceCommandMessageUnionV4,
  },
};

use super::server_device::{ServerDevice, ServerDeviceEvent};

/// Commands that can be sent to a device through its handle.
///
/// Each command variant includes a oneshot channel for returning the result
/// back to the caller.
#[derive(Debug)]
pub enum DeviceCommand {
  /// Output command (vibrate, rotate, oscillate, etc.)
  Output {
    cmd: CheckedOutputCmdV4,
    response: oneshot::Sender<Result<(), ButtplugError>>,
  },
  /// Input command (read sensor, subscribe/unsubscribe, etc.)
  Input {
    cmd: CheckedInputCmdV4,
    response: oneshot::Sender<Result<(), ButtplugError>>,
  },
  /// Stop all device outputs and optionally unsubscribe from inputs
  Stop {
    stop_outputs: bool,
    stop_inputs: bool,
    response: oneshot::Sender<Result<(), ButtplugError>>,
  },
  /// Disconnect the device
  Disconnect,
}

/// Lightweight handle for communicating with a device.
///
/// DeviceHandle is a facade that wraps the underlying device implementation.
/// It provides a clean interface for sending commands and querying device
/// properties without exposing internal implementation details.
///
/// DeviceHandle is cheap to clone and can be safely shared across tasks.
#[derive(Clone)]
pub struct DeviceHandle {
  // Currently wraps ServerDevice - will be migrated to channel-based communication
  device: Arc<ServerDevice>,
}

impl DeviceHandle {
  /// Create a new DeviceHandle wrapping a ServerDevice
  pub fn new(device: Arc<ServerDevice>) -> Self {
    Self { device }
  }

  /// Get the device's unique identifier
  pub fn identifier(&self) -> &UserDeviceIdentifier {
    self.device.identifier()
  }

  /// Get the device's name
  pub fn name(&self) -> String {
    self.device.name()
  }

  /// Get the device's definition (contains features, display name, etc.)
  pub fn definition(&self) -> &ServerDeviceDefinition {
    self.device.definition()
  }

  /// Get the device's legacy attributes (for older API compatibility)
  pub fn legacy_attributes(&self) -> &ServerDeviceAttributes {
    self.device.legacy_attributes()
  }

  /// Get the device as a DeviceMessageInfoV4 for protocol messages
  pub fn as_device_message_info(&self, index: u32) -> DeviceMessageInfoV4 {
    self.device.as_device_message_info(index)
  }

  /// Parse and handle a command message for this device
  pub fn parse_message(&self, command_message: ButtplugDeviceCommandMessageUnionV4) -> ButtplugServerResultFuture {
    self.device.parse_message(command_message)
  }

  /// Disconnect from the device
  pub fn disconnect(&self) -> ButtplugResultFuture {
    self.device.disconnect()
  }

  /// Get the event stream for this device (connections, disconnections, notifications)
  pub fn event_stream(&self) -> impl futures::Stream<Item = ServerDeviceEvent> + Send + use<> {
    self.device.event_stream()
  }

  /// Get access to the underlying ServerDevice
  ///
  /// This is a temporary method during migration. It will be removed once
  /// all device communication goes through DeviceCommand channels.
  pub fn inner(&self) -> &Arc<ServerDevice> {
    &self.device
  }
}

impl std::fmt::Debug for DeviceHandle {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("DeviceHandle")
      .field("identifier", self.device.identifier())
      .field("name", &self.device.name())
      .finish()
  }
}
