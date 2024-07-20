// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
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
pub struct RotationSubcommandV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  feature_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Speed"))]
  speed: f64,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Clockwise"))]
  clockwise: bool,
}

impl RotationSubcommandV4 {
  pub fn new(feature_index: u32, speed: f64, clockwise: bool) -> Self {
    Self {
      feature_index,
      speed,
      clockwise,
    }
  }
}

#[derive(Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, Getters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct RotateCmdV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[getset(get = "pub")]
  #[cfg_attr(feature = "serialize-json", serde(rename = "Rotations"))]
  #[getset(get = "pub")]
  rotations: Vec<RotationSubcommandV4>,
}

impl RotateCmdV4 {
  pub fn new(device_index: u32, rotations: Vec<RotationSubcommandV4>) -> Self {
    Self {
      id: 1,
      device_index,
      rotations,
    }
  }
}

impl ButtplugMessageValidator for RotateCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    for rotation in &self.rotations {
      self.is_in_command_range(
        rotation.speed,
        format!(
          "Speed {} for RotateCmd index {} is invalid. Speed should be a value between 0.0 and 1.0",
          rotation.speed, rotation.feature_index
        ),
      )?;
    }
    Ok(())
  }
}

#[derive(Debug, PartialEq, Clone, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
#[getset(get_copy = "pub")]
pub struct RotationSubcommandV2 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Speed"))]
  speed: f64,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Clockwise"))]
  clockwise: bool,
}

impl RotationSubcommandV2 {
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
pub struct RotateCmdV2 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[getset(get = "pub")]
  #[cfg_attr(feature = "serialize-json", serde(rename = "Rotations"))]
  #[getset(get = "pub")]
  rotations: Vec<RotationSubcommandV2>,
}

impl RotateCmdV2 {
  pub fn new(device_index: u32, rotations: Vec<RotationSubcommandV2>) -> Self {
    Self {
      id: 1,
      device_index,
      rotations,
    }
  }
}

impl ButtplugMessageValidator for RotateCmdV2 {
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
