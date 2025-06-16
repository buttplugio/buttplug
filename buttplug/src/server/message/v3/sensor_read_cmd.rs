// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
    message::{
      ButtplugDeviceMessage,
      ButtplugMessage,
      ButtplugMessageFinalizer,
      ButtplugMessageValidator,
      SensorCommandType,
      SensorType,
    },
  },
  server::message::{
    checked_sensor_cmd::CheckedSensorCmdV4,
    ServerDeviceAttributes,
    TryFromDeviceAttributes,
  },
};
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

#[derive(
  Debug,
  ButtplugDeviceMessage,
  ButtplugMessageFinalizer,
  PartialEq,
  Eq,
  Clone,
  Getters,
  CopyGetters,
  Serialize,
  Deserialize,
)]
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
  sensor_type: SensorType,
}

impl SensorReadCmdV3 {
  pub fn new(device_index: u32, sensor_index: u32, sensor_type: SensorType) -> Self {
    Self {
      id: 1,
      device_index,
      sensor_index,
      sensor_type,
    }
  }
}

impl ButtplugMessageValidator for SensorReadCmdV3 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
    // TODO Should expected_length always be > 0?
  }
}

impl TryFromDeviceAttributes<SensorReadCmdV3> for CheckedSensorCmdV4 {
  fn try_from_device_attributes(
    msg: SensorReadCmdV3,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    // Reject any SensorRead that's not a battery, we never supported sensors otherwise in v3.
    if msg.sensor_type != SensorType::Battery {
      Err(ButtplugError::from(
        ButtplugDeviceError::MessageNotSupported("SensorReadCmdV3".to_owned()),
      ))
    } else if let Some((feature_index, feature)) =
      features.features().iter().enumerate().find(|(_, p)| {
        if let Some(sensor_map) = p.sensor() {
          if sensor_map.contains_key(&SensorType::Battery) {
            return true;
          }
        }
        false
      })
    {
      Ok(CheckedSensorCmdV4::new(
        msg.device_index(),
        feature_index as u32,
        SensorType::Battery,
        SensorCommandType::Read,
        feature.id(),
      ))
    } else {
      Err(ButtplugError::from(
        ButtplugDeviceError::MessageNotSupported("SensorReadCmdV3".to_owned()),
      ))
    }
  }
}
