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
      SensorType,
    },
  },
  server::message::{
    checked_sensor_read_cmd::CheckedSensorReadCmdV4,
    ServerDeviceAttributes,
    TryFromDeviceAttributes,
  },
};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

/// Battery level request
#[derive(Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct BatteryLevelCmdV2 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
}

impl BatteryLevelCmdV2 {
  pub fn new(device_index: u32) -> Self {
    Self {
      id: 1,
      device_index,
    }
  }
}

impl ButtplugMessageValidator for BatteryLevelCmdV2 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}

impl TryFromDeviceAttributes<BatteryLevelCmdV2> for CheckedSensorReadCmdV4 {
  fn try_from_device_attributes(
    msg: BatteryLevelCmdV2,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let battery_feature = features
      .attrs_v2()
      .battery_level_cmd()
      .as_ref()
      .ok_or(ButtplugError::from(
        ButtplugDeviceError::DeviceConfigurationError(
          "Device configuration does not have Battery sensor available.".to_owned(),
        ),
      ))?
      .feature();

    Ok(CheckedSensorReadCmdV4::new(
      msg.device_index(),
      0,
      SensorType::Battery,
      battery_feature.id(),
    ))
  }
}
