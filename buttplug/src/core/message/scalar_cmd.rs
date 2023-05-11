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

/// Generic command for setting a level (single magnitude value) of a device feature.
#[derive(Debug, PartialEq, Clone, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
#[getset(get_copy = "pub")]
pub struct ScalarSubcommand {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Scalar"))]
  scalar: f64,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ActuatorType"))]
  actuator_type: ActuatorType,
}

impl ScalarSubcommand {
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
pub struct ScalarCmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Scalars"))]
  #[getset(get = "pub")]
  scalars: Vec<ScalarSubcommand>,
}

impl ScalarCmd {
  pub fn new(device_index: u32, scalars: Vec<ScalarSubcommand>) -> Self {
    Self {
      id: 1,
      device_index,
      scalars,
    }
  }
}

impl ButtplugMessageValidator for ScalarCmd {
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

impl From<VibrateCmd> for ScalarCmd {
  fn from(vibrate_cmd: VibrateCmd) -> Self {
    let subcommands: Vec<ScalarSubcommand> = vibrate_cmd
      .speeds()
      .iter()
      .map(|x| ScalarSubcommand::new(x.index(), x.speed(), ActuatorType::Vibrate))
      .collect();
    Self {
      id: vibrate_cmd.id(),
      device_index: vibrate_cmd.device_index(),
      scalars: subcommands,
    }
  }
}
