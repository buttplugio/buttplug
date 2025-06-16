// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::{
  errors::ButtplugMessageError,
  message::{
    ButtplugDeviceMessage,
    ButtplugMessage,
    ButtplugMessageFinalizer,
    ButtplugMessageValidator,
    SensorType,
  },
};
use getset::Getters;
use serde::{Deserialize, Serialize};

#[derive(
  Debug,
  ButtplugDeviceMessage,
  ButtplugMessageFinalizer,
  PartialEq,
  Eq,
  Clone,
  Getters,
  Serialize,
  Deserialize,
)]
pub struct SensorSubscribeCmdV3 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[getset(get = "pub")]
  #[serde(rename = "SensorIndex")]
  sensor_index: u32,
  #[getset(get = "pub")]
  #[serde(rename = "SensorType")]
  sensor_type: SensorType,
}

impl SensorSubscribeCmdV3 {
  pub fn new(device_index: u32, sensor_index: u32, sensor_type: SensorType) -> Self {
    Self {
      id: 1,
      device_index,
      sensor_index,
      sensor_type,
    }
  }
}

impl ButtplugMessageValidator for SensorSubscribeCmdV3 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
