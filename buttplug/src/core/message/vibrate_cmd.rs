// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Clone, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
#[getset(get_copy = "pub")]
pub struct VibrateSubcommand {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Speed"))]
  speed: f64,
}

impl VibrateSubcommand {
  pub fn new(index: u32, speed: f64) -> Self {
    Self { index, speed }
  }
}

#[derive(
  Debug, Default, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, Getters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct VibrateCmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Speeds"))]
  #[getset(get = "pub")]
  speeds: Vec<VibrateSubcommand>,
}

impl VibrateCmd {
  pub fn new(device_index: u32, speeds: Vec<VibrateSubcommand>) -> Self {
    Self {
      id: 1,
      device_index,
      speeds,
    }
  }
}

impl ButtplugMessageValidator for VibrateCmd {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    for speed in &self.speeds {
      self.is_in_command_range(speed.speed, format!("Speed {} for VibrateCmd index {} is invalid. Speed should be a value between 0.0 and 1.0", speed.speed, speed.index))?;
    }
    Ok(())
  }
}
