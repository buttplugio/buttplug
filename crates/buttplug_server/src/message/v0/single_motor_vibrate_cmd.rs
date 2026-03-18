// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::{
  errors::ButtplugMessageError,
  message::{ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageValidator},
};
use getset::CopyGetters;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, CopyGetters, Serialize, Deserialize)]
pub struct SingleMotorVibrateCmdV0 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[serde(rename = "Speed")]
  #[getset(get_copy = "pub")]
  speed: f64,
}

impl SingleMotorVibrateCmdV0 {
  pub fn new(device_index: u32, speed: f64) -> Self {
    Self {
      id: 1,
      device_index,
      speed,
    }
  }
}

impl ButtplugMessage for SingleMotorVibrateCmdV0 {
  fn id(&self) -> u32 {
    self.id
  }
  fn set_id(&mut self, id: u32) {
    self.id = id;
  }
}

impl ButtplugDeviceMessage for SingleMotorVibrateCmdV0 {
  fn device_index(&self) -> u32 {
    self.device_index
  }
  fn set_device_index(&mut self, device_index: u32) {
    self.device_index = device_index;
  }
}

impl ButtplugMessageValidator for SingleMotorVibrateCmdV0 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    self.is_in_command_range(
      self.speed,
      format!(
        "SingleMotorVibrateCmd Speed {} is invalid. Valid speeds are 0.0-1.0.",
        self.speed
      ),
    )
  }
}
