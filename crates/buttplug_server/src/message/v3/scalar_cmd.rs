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
    OutputType,
  },
};
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

/// Generic command for setting a level (single magnitude value) of a device feature.
#[derive(Debug, PartialEq, Clone, CopyGetters, Serialize, Deserialize)]
#[getset(get_copy = "pub")]
pub struct ScalarSubcommandV3 {
  #[serde(rename = "Index")]
  index: u32,
  #[serde(rename = "Scalar")]
  scalar: f64,
  #[serde(rename = "ActuatorType")]
  actuator_type: OutputType,
}

impl ScalarSubcommandV3 {
  pub fn new(index: u32, scalar: f64, actuator_type: OutputType) -> Self {
    Self {
      index,
      scalar,
      actuator_type,
    }
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
pub struct ScalarCmdV3 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[serde(rename = "Scalars")]
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
