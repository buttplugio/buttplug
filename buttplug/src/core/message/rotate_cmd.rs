// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
pub use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
#[getset(get_copy = "pub")]
pub struct RotationSubcommand {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Speed"))]
  speed: f64,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Clockwise"))]
  clockwise: bool,
}

impl RotationSubcommand {
  pub fn new(index: u32, speed: f64, clockwise: bool) -> Self {
    Self {
      index,
      speed,
      clockwise,
    }
  }
}

#[derive(Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, Getters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct RotateCmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[getset(get = "pub")]
  #[cfg_attr(feature = "serialize-json", serde(rename = "Rotations"))]
  #[getset(get = "pub")]
  rotations: Vec<RotationSubcommand>,
}

impl RotateCmd {
  pub fn new(device_index: u32, rotations: Vec<RotationSubcommand>) -> Self {
    Self {
      id: 1,
      device_index,
      rotations,
    }
  }
}

impl ButtplugMessageValidator for RotateCmd {
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
