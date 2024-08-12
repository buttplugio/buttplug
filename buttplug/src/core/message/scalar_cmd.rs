// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

/// Generic command for setting a level (single magnitude value) of a device feature.
#[derive(Debug, PartialEq, Clone, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
#[getset(get_copy = "pub")]
pub struct ScalarSubcommandV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  feature_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Scalar"))]
  scalar: f64,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ActuatorType"))]
  actuator_type: ActuatorType,
}

impl ScalarSubcommandV4 {
  pub fn new(feature_index: u32, scalar: f64, actuator_type: ActuatorType) -> Self {
    Self {
      feature_index,
      scalar,
      actuator_type,
    }
  }
}

#[derive(
  Debug, Default, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, Getters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct ScalarCmdV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Scalars"))]
  #[getset(get = "pub")]
  scalars: Vec<ScalarSubcommandV4>,
}

impl ScalarCmdV4 {
  pub fn new(device_index: u32, scalars: Vec<ScalarSubcommandV4>) -> Self {
    Self {
      id: 1,
      device_index,
      scalars,
    }
  }
}

impl ButtplugMessageValidator for ScalarCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    for level in &self.scalars {
      self.is_in_command_range(
        level.scalar,
        format!(
          "Level {} for ScalarCmd feature index {} is invalid. Level should be a value between 0.0 and 1.0",
          level.scalar, level.feature_index
        ),
      )?;
    }
    Ok(())
  }
}

/// Generic command for setting a level (single magnitude value) of a device feature.
#[derive(Debug, PartialEq, Clone, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
#[getset(get_copy = "pub")]
pub struct ScalarSubcommandV3 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Scalar"))]
  scalar: f64,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ActuatorType"))]
  actuator_type: ActuatorType,
}

impl ScalarSubcommandV3 {
  pub fn new(index: u32, scalar: f64, actuator_type: ActuatorType) -> Self {
    Self {
      index,
      scalar,
      actuator_type,
    }
  }
}

#[derive(
  Debug, Default, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, Getters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct ScalarCmdV3 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Scalars"))]
  #[getset(get = "pub")]
  scalars: Vec<ScalarSubcommandV3>,
}

impl ScalarCmdV3 {
  pub fn new(device_index: u32, scalars: Vec<ScalarSubcommandV3>) -> Self {
    Self {
      id: 1,
      device_index,
      scalars,
    }
  }
}

impl ButtplugMessageValidator for ScalarCmdV3 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    for level in &self.scalars {
      self.is_in_command_range(
        level.scalar,
        format!(
          "Level {} for ScalarCmd index {} is invalid. Level should be a value between 0.0 and 1.0",
          level.scalar, level.index
        ),
      )?;
    }
    Ok(())
  }
}
