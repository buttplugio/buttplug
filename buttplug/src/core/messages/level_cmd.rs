// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct LevelSubcommand {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Level"))]
  level: f64,
}

impl LevelSubcommand {
  pub fn new(index: u32, level: f64) -> Self {
    Self { index, level }
  }

  pub fn index(&self) -> u32 {
    self.index
  }

  pub fn level(&self) -> f64 {
    self.level
  }
}

#[derive(Debug, Default, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct LevelCmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Levels"))]
  levels: Vec<LevelSubcommand>,
}

impl LevelCmd {
  pub fn new(device_index: u32, levels: Vec<LevelSubcommand>) -> Self {
    Self {
      id: 1,
      device_index,
      levels,
    }
  }

  pub fn levels(&self) -> &Vec<LevelSubcommand> {
    &self.levels
  }
}

impl ButtplugMessageValidator for LevelCmd {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    for level in &self.levels {
      self.is_in_command_range(level.level, format!("Level {} for LevelCmd index {} is invalid. Level should be a value between 0.0 and 1.0", level.level, level.index))?;
    }
    Ok(())
  }
}
