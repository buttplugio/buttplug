// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::{
  errors::ButtplugMessageError,
  message::{
    ButtplugDeviceMessage,
    ButtplugMessage,
    ButtplugMessageFinalizer,
    ButtplugMessageValidator,
  },
};
use getset::CopyGetters;
use serde::{Deserialize, Serialize};

/// Battery level response
#[derive(
  Debug,
  ButtplugDeviceMessage,
  ButtplugMessageFinalizer,
  PartialEq,
  Clone,
  CopyGetters,
  Serialize,
  Deserialize,
)]
pub struct BatteryLevelReadingV2 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[serde(rename = "BatteryLevel")]
  #[getset(get_copy = "pub")]
  battery_level: f64,
}

impl BatteryLevelReadingV2 {
  pub fn new(device_index: u32, battery_level: f64) -> Self {
    Self {
      id: 1,
      device_index,
      battery_level,
    }
  }
}

impl ButtplugMessageValidator for BatteryLevelReadingV2 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    self.is_in_command_range(
      self.battery_level,
      "BatteryLevelReading must be between 0.0 and 1.0".to_string(),
    )
  }
}
