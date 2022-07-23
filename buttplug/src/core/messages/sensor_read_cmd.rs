// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};
use getset::{CopyGetters, Getters};

#[derive(Debug, PartialEq, Eq, Clone, Getters)]
#[getset(get="pub")]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct SensorSubcommand {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "SensorType"))]
  sensor_type: SensorType,
}

impl SensorSubcommand {
  pub fn new(index: u32, sensor_type: SensorType) -> Self {
    Self { index, sensor_type }
  }
}


#[derive(Debug, ButtplugDeviceMessage, PartialEq, Eq, Clone, Getters, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct SensorReadCmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Sensors"))]
  sensors: Vec<SensorSubcommand>
}

impl SensorReadCmd {
  pub fn new(device_index: u32, sensors: Vec<SensorSubcommand>) -> Self {
    Self {
      id: 1,
      device_index,
      sensors
    }
  }
}

impl ButtplugMessageValidator for SensorReadCmd {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
    // TODO Should expected_length always be > 0?
  }
}