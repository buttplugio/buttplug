// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::{
  ButtplugDeviceMessage,
  ButtplugMessage,
  ButtplugMessageError,
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
};
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

/// Generic command for setting a level (single magnitude value) of a device feature.
#[derive(Debug, PartialEq, Clone, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
#[getset(get_copy = "pub")]
pub struct LevelSubcommandV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  feature_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Scalar"))]
  level: i32,
}

impl LevelSubcommandV4 {
  pub fn new(feature_index: u32, level: i32) -> Self {
    Self {
      feature_index,
      level,
    }
  }
}

#[derive(
  Debug, Default, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, Getters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct LevelCmdV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Scalars"))]
  #[getset(get = "pub")]
  levels: Vec<LevelSubcommandV4>,
}

impl LevelCmdV4 {
  pub fn new(device_index: u32, levels: Vec<LevelSubcommandV4>) -> Self {
    Self {
      id: 1,
      device_index,
      levels,
    }
  }
}

impl ButtplugMessageValidator for LevelCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    Ok(())
  }
}
