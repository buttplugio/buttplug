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
  SensorType,
};
use getset::CopyGetters;
use serde::{Deserialize, Serialize};


#[derive(
  Debug, Display, PartialEq, Eq, Clone, Serialize, Deserialize, Hash, Copy
)]
pub enum SensorCommandType {
  Read,
  Subscribe,
  Unsubscribe
}

#[derive(
  Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone, Copy, CopyGetters, Serialize, Deserialize
)]
pub struct SensorCmdV4 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[getset(get_copy = "pub")]
  #[serde(rename = "FeatureIndex")]
  feature_index: u32,
  #[getset(get_copy = "pub")]
  #[serde(rename = "SensorType")]
  sensor_type: SensorType,
  #[getset(get_copy = "pub")]
  #[serde(rename = "SensorCommandType")]
  sensor_command_type: SensorCommandType,
}

impl SensorCmdV4 {
  pub fn new(device_index: u32, feature_index: u32, sensor_type: SensorType, sensor_command_type: SensorCommandType) -> Self {
    Self {
      id: 1,
      device_index,
      feature_index,
      sensor_type,
      sensor_command_type
    }
  }
}

impl ButtplugMessageValidator for SensorCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
    // TODO Should expected_length always be > 0?
  }
}
