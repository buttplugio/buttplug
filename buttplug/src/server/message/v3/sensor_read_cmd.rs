// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugMessageError,
    message::{
      ButtplugDeviceMessage,
      ButtplugMessage,
      ButtplugMessageFinalizer,
      ButtplugMessageValidator,
      SensorType,
    },
  },
  server::message::{
    checked_sensor_read_cmd::CheckedSensorReadCmdV4,
    LegacyDeviceAttributes,
    TryFromDeviceAttributes,
  },
};
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(
  Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone, Getters, CopyGetters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct SensorReadCmdV3 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[getset(get = "pub")]
  #[cfg_attr(feature = "serialize-json", serde(rename = "SensorIndex"))]
  sensor_index: u32,
  #[getset(get = "pub")]
  #[cfg_attr(feature = "serialize-json", serde(rename = "SensorType"))]
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

impl TryFromDeviceAttributes<SensorReadCmdV3> for CheckedSensorReadCmdV4 {
  fn try_from_device_attributes(
    msg: SensorReadCmdV3,
    features: &LegacyDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let sensor_feature_id = features.attrs_v3().sensor_read_cmd().as_ref().unwrap()
      [*msg.sensor_index() as usize]
      .feature()
      .id();

    Ok(CheckedSensorReadCmdV4::new(
      msg.device_index(),
      0,
      *msg.sensor_type(),
      *sensor_feature_id,
    ))
  }
}