// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::{
  find_device_feature_indexes, ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageError, ButtplugMessageFinalizer, ButtplugMessageValidator, ButtplugSensorFeatureMessageType, SensorSubscribeCmdV3, SensorType, TryFromDeviceFeatures
};
use getset::Getters;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone, Getters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct SensorSubscribeCmdV4 {
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
}

impl SensorSubscribeCmdV4 {
  pub fn new(device_index: u32, feature_index: u32, sensor_type: SensorType) -> Self {
    Self {
      id: 1,
      device_index,
      feature_index,
      sensor_type,
    }
  }
}

impl ButtplugMessageValidator for SensorSubscribeCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}


impl TryFromDeviceFeatures<SensorSubscribeCmdV3> for SensorSubscribeCmdV4 {
  fn try_from_device_features(msg: SensorSubscribeCmdV3, features: &[crate::core::message::DeviceFeature]) -> Result<Self, crate::core::errors::ButtplugError> {
    let features = find_device_feature_indexes(features, |(_, x)| {
      x.sensor().as_ref().is_some_and(|y| {
        y.messages()
          .contains(&ButtplugSensorFeatureMessageType::SensorSubscribeCmd)
      })
    })?;
  
    let sensor_feature_index = features[*msg.sensor_index() as usize] as u32;
  
    Ok(
      SensorSubscribeCmdV4::new(
        msg.device_index(),
        sensor_feature_index,
        *msg.sensor_type(),
      )
      .into(),
    )
  }
}
