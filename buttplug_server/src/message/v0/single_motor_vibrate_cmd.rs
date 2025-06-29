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
