// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::{
  errors::ButtplugMessageError,
  message::{ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageValidator, InputType},
};
use getset::Getters;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Getters, Serialize, Deserialize)]
pub struct SensorUnsubscribeCmdV3 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[serde(rename = "SensorIndex")]
  #[getset(get = "pub")]
  sensor_index: u32,
  #[serde(rename = "SensorType")]
  #[getset(get = "pub")]
  sensor_type: InputType,
}

impl SensorUnsubscribeCmdV3 {
  pub fn new(device_index: u32, sensor_index: u32, sensor_type: InputType) -> Self {
    Self {
      id: 1,
      device_index,
      sensor_index,
      sensor_type,
    }
  }
}

impl ButtplugMessage for SensorUnsubscribeCmdV3 {
  fn id(&self) -> u32 {
    self.id
  }
  fn set_id(&mut self, id: u32) {
    self.id = id;
  }
}

impl ButtplugDeviceMessage for SensorUnsubscribeCmdV3 {
  fn device_index(&self) -> u32 {
    self.device_index
  }
  fn set_device_index(&mut self, device_index: u32) {
    self.device_index = device_index;
  }
}

impl ButtplugMessageValidator for SensorUnsubscribeCmdV3 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
