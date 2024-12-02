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
  server::message::{internal_sensor_read_cmd::InternalSensorReadCmdV4, LegacyDeviceAttributes, TryFromDeviceAttributes},
};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct RSSILevelCmdV2 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
}

impl RSSILevelCmdV2 {
  pub fn new(device_index: u32) -> Self {
    Self {
      id: 1,
      device_index,
    }
  }
}

impl ButtplugMessageValidator for RSSILevelCmdV2 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}

impl TryFromDeviceAttributes<RSSILevelCmdV2> for InternalSensorReadCmdV4 {
  fn try_from_device_attributes(
    msg: RSSILevelCmdV2,
    features: &LegacyDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let rssi_feature = features
      .attrs_v2()
      .rssi_level_cmd()
      .as_ref()
      .ok_or(ButtplugError::from(
        ButtplugDeviceError::DeviceConfigurationError(
          "Device configuration does not have Battery sensor available.".to_owned(),
        ),
      ))?
      .feature();

    Ok(
      InternalSensorReadCmdV4::new(
        msg.device_index(),
        0,
        SensorType::RSSI,
        *rssi_feature.id(),
      )
      .into(),
    )
  }
}
