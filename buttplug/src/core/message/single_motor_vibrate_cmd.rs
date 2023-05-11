// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
use getset::CopyGetters;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct SingleMotorVibrateCmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Speed"))]
  #[getset(get_copy = "pub")]
  speed: f64,
}

impl SingleMotorVibrateCmd {
  pub fn new(device_index: u32, speed: f64) -> Self {
    Self {
      id: 1,
      device_index,
      speed,
    }
  }
}

impl ButtplugMessageValidator for SingleMotorVibrateCmd {
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
