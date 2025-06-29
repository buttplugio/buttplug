// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  ButtplugMessage,
  ButtplugMessageError,
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
  DeviceFeature,
};
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

use super::DeviceMessageInfoV4;

/// Notification that a device has been found and connected to the server.
#[derive(
  ButtplugMessage, Clone, Debug, PartialEq, Getters, CopyGetters, Serialize, Deserialize,
)]
pub struct DeviceAddedV4 {
  #[serde(rename = "Id")]
  id: u32,
  // DeviceAdded is not considered a device message because it only notifies of existence and is not
  // a command (and goes from server to client), therefore we have to define the getter ourselves.
  #[serde(rename = "DeviceIndex")]
  #[getset(get_copy = "pub")]
  device_index: u32,
  #[serde(rename = "DeviceName")]
  #[getset(get = "pub")]
  device_name: String,
  #[serde(rename = "DeviceDisplayName", skip_serializing_if = "Option::is_none")]
  #[getset(get = "pub")]
  device_display_name: Option<String>,
  #[serde(rename = "DeviceMessageTimingGap")]
  #[getset(get_copy = "pub")]
  device_message_timing_gap: u32,
  #[serde(rename = "DeviceFeatures")]
  #[getset(get = "pub")]
  device_features: Vec<DeviceFeature>,
}

impl DeviceAddedV4 {
  pub fn new(
    device_index: u32,
    device_name: &str,
    device_display_name: &Option<String>,
    device_message_timing_gap: u32,
    device_features: &Vec<DeviceFeature>,
  ) -> Self {
    let mut obj = Self {
      id: 0,
      device_index,
      device_name: device_name.to_string(),
      device_display_name: device_display_name.clone(),
      device_message_timing_gap,
      device_features: device_features.clone(),
    };
    obj.finalize();
    obj
  }
}

impl From<DeviceMessageInfoV4> for DeviceAddedV4 {
  fn from(value: DeviceMessageInfoV4) -> Self {
    Self {
      id: 0,
      device_index: value.device_index(),
      device_name: value.device_name().clone(),
      device_display_name: value.device_display_name().clone(),
      device_message_timing_gap: value.device_message_timing_gap(),
      device_features: value.device_features().clone(),
    }
  }
}

impl ButtplugMessageValidator for DeviceAddedV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_system_id(self.id)
  }
}

impl ButtplugMessageFinalizer for DeviceAddedV4 {
  fn finalize(&mut self) {
  }
}
