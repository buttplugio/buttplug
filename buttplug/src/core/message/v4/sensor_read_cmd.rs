// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::{errors::{ButtplugDeviceError, ButtplugError}, message::{
  BatteryLevelCmdV2, ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageError, ButtplugMessageFinalizer, ButtplugMessageValidator, LegacyDeviceAttributes, RSSILevelCmdV2, SensorReadCmdV3, SensorType, TryFromDeviceAttributes
}};
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(
  Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone, Getters, CopyGetters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct SensorReadCmdV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[getset(get = "pub")]
  #[cfg_attr(feature = "serialize-json", serde(rename = "FeatureIndex"))]
  feature_index: u32,
  #[getset(get = "pub")]
  #[cfg_attr(feature = "serialize-json", serde(rename = "SensorType"))]
  sensor_type: SensorType,
  #[getset(get = "pub")]
  #[cfg_attr(feature = "serialize-json", serde(skip))]
  feature_id: Option<Uuid>
}

impl SensorReadCmdV4 {
  pub fn new(device_index: u32, feature_index: u32, sensor_type: SensorType, feature_id: &Option<Uuid>) -> Self {
    Self {
      id: 1,
      device_index,
      feature_index,
      sensor_type,
      feature_id: feature_id.clone()
    }
  }
}

impl ButtplugMessageValidator for SensorReadCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
    // TODO Should expected_length always be > 0?
  }
}

impl TryFromDeviceAttributes<BatteryLevelCmdV2> for SensorReadCmdV4 {
  fn try_from_device_attributes(msg: BatteryLevelCmdV2, features: &LegacyDeviceAttributes) -> Result<Self, crate::core::errors::ButtplugError> {
    let battery_feature = features.attrs_v2().battery_level_cmd().as_ref().ok_or(ButtplugError::from(ButtplugDeviceError::DeviceConfigurationError("Device configuration does not have Battery sensor available.".to_owned())))?.feature();

    Ok(
      SensorReadCmdV4::new(
        msg.device_index(),
        0,
        SensorType::Battery,
        &Some(battery_feature.id().clone())
      )
      .into(),
    )
  }
}

impl TryFromDeviceAttributes<RSSILevelCmdV2> for SensorReadCmdV4 {
  fn try_from_device_attributes(msg: RSSILevelCmdV2, features: &LegacyDeviceAttributes) -> Result<Self, crate::core::errors::ButtplugError> {
    let rssi_feature = features.attrs_v2().rssi_level_cmd().as_ref().ok_or(ButtplugError::from(ButtplugDeviceError::DeviceConfigurationError("Device configuration does not have Battery sensor available.".to_owned())))?.feature();

    Ok(
      SensorReadCmdV4::new(
        msg.device_index(),
        0,
        SensorType::RSSI,
        &Some(rssi_feature.id().clone())
      )
      .into(),
    )
  }
}

impl TryFromDeviceAttributes<SensorReadCmdV3> for SensorReadCmdV4 {
  fn try_from_device_attributes(msg: SensorReadCmdV3, features: &LegacyDeviceAttributes) -> Result<Self, crate::core::errors::ButtplugError> {
    let sensor_feature_id = features.attrs_v3().sensor_read_cmd().as_ref().unwrap()[*msg.sensor_index() as usize].feature().id();
  
    Ok(
      SensorReadCmdV4::new(
        msg.device_index(),
        0,
        *msg.sensor_type(),
        &Some(sensor_feature_id.clone())
      )
      .into(),
    )
  }
}
