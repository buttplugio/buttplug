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
pub use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, CopyGetters, Serialize, Deserialize)]
#[getset(get_copy = "pub")]
pub struct RotationSubcommandV1 {
  #[serde(rename = "Index")]
  index: u32,
  #[serde(rename = "Speed")]
  speed: f64,
  #[serde(rename = "Clockwise")]
  clockwise: bool,
}

impl RotationSubcommandV1 {
  pub fn new(index: u32, speed: f64, clockwise: bool) -> Self {
    Self {
      index,
      speed,
      clockwise,
    }
  }
}

#[derive(
  Debug,
  ButtplugDeviceMessage,
  ButtplugMessageFinalizer,
  PartialEq,
  Clone,
  Getters,
  Serialize,
  Deserialize,
)]
pub struct RotateCmdV1 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[getset(get = "pub")]
  #[serde(rename = "Rotations")]
  #[getset(get = "pub")]
  rotations: Vec<RotationSubcommandV1>,
}

impl RotateCmdV1 {
  pub fn new(device_index: u32, rotations: Vec<RotationSubcommandV1>) -> Self {
    Self {
      id: 1,
      device_index,
      rotations,
    }
  }
}

impl ButtplugMessageValidator for RotateCmdV1 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    for rotation in &self.rotations {
      self.is_in_command_range(
        rotation.speed,
        format!(
          "Speed {} for RotateCmd index {} is invalid. Speed should be a value between 0.0 and 1.0",
          rotation.speed, rotation.index
        ),
      )?;
    }
    Ok(())
  }
}
