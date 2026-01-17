// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  ServerDeviceAttributes,
  TryFromDeviceAttributes,
  checked_input_cmd::CheckedInputCmdV4,
};
use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
  message::{
    ButtplugDeviceMessage,
    ButtplugMessage,
    ButtplugMessageValidator,
    InputCommandType,
    InputType,
  },
};
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Getters, CopyGetters, Serialize, Deserialize)]
pub struct SensorReadCmdV3 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[getset(get = "pub")]
  #[serde(rename = "SensorIndex")]
  sensor_index: u32,
  #[getset(get = "pub")]
  #[serde(rename = "SensorType")]
  sensor_type: InputType,
}

impl SensorReadCmdV3 {
  pub fn new(device_index: u32, sensor_index: u32, sensor_type: InputType) -> Self {
    Self {
      id: 1,
      device_index,
      sensor_index,
      sensor_type,
    }
  }
}

impl ButtplugMessage for SensorReadCmdV3 {
  fn id(&self) -> u32 {
    self.id
  }
  fn set_id(&mut self, id: u32) {
    self.id = id;
  }
}

impl ButtplugDeviceMessage for SensorReadCmdV3 {
  fn device_index(&self) -> u32 {
    self.device_index
  }
  fn set_device_index(&mut self, device_index: u32) {
    self.device_index = device_index;
  }
}

impl ButtplugMessageValidator for SensorReadCmdV3 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
    // TODO Should expected_length always be > 0?
  }
}

impl TryFromDeviceAttributes<SensorReadCmdV3> for CheckedInputCmdV4 {
  fn try_from_device_attributes(
    msg: SensorReadCmdV3,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, buttplug_core::errors::ButtplugError> {
    // Reject any SensorRead that's not a battery, we never supported sensors otherwise in v3.
    if msg.sensor_type != InputType::Battery {
      Err(ButtplugError::from(
        ButtplugDeviceError::MessageNotSupported("SensorReadCmdV3".to_owned()),
      ))
    } else if let Some((feature_index, feature)) = features
      .features()
      .iter()
      .find(|(_, p)| p.input().as_ref().is_some_and(|x| x.battery().is_some()))
    {
      Ok(CheckedInputCmdV4::new(
        msg.id(),
        msg.device_index(),
        *feature_index,
        InputType::Battery,
        InputCommandType::Read,
        feature.id(),
      ))
    } else {
      Err(ButtplugError::from(
        ButtplugDeviceError::MessageNotSupported("SensorReadCmdV3".to_owned()),
      ))
    }
  }
}
