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
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Clone, CopyGetters, Serialize, Deserialize)]
#[getset(get_copy = "pub")]
pub struct VibrateSubcommandV1 {
  #[serde(rename = "Index")]
  index: u32,
  #[serde(rename = "Speed")]
  speed: f64,
}

impl VibrateSubcommandV1 {
  pub fn new(index: u32, speed: f64) -> Self {
    Self { index, speed }
  }
}

#[derive(
  Debug,
  Default,
  ButtplugDeviceMessage,
  ButtplugMessageFinalizer,
  PartialEq,
  Clone,
  Getters,
  Serialize,
  Deserialize,
)]
pub struct VibrateCmdV1 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[serde(rename = "Speeds")]
  #[getset(get = "pub")]
  speeds: Vec<VibrateSubcommandV1>,
}

impl VibrateCmdV1 {
  pub fn new(device_index: u32, speeds: Vec<VibrateSubcommandV1>) -> Self {
    Self {
      id: 1,
      device_index,
      speeds,
    }
  }
}

impl ButtplugMessageValidator for VibrateCmdV1 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    for speed in &self.speeds {
      self.is_in_command_range(speed.speed, format!("Speed {} for VibrateCmd index {} is invalid. Speed should be a value between 0.0 and 1.0", speed.speed, speed.index))?;
    }
    Ok(())
  }
}
