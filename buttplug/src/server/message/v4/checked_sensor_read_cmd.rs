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
      SensorReadCmdV4,
      SensorType,
    },
  },
  server::message::TryFromDeviceAttributes,
};
use getset::CopyGetters;
use uuid::Uuid;

#[derive(
  Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone, CopyGetters,
)]
#[getset(get_copy = "pub")]
pub struct CheckedSensorReadCmdV4 {
  id: u32,
  device_index: u32,
  feature_index: u32,
  sensor_type: SensorType,
  feature_id: Uuid,
}

impl CheckedSensorReadCmdV4 {
  pub fn new(
    device_index: u32,
    feature_index: u32,
    sensor_type: SensorType,
    feature_id: Uuid,
  ) -> Self {
    Self {
      id: 1,
      device_index,
      feature_index,
      sensor_type,
      feature_id,
    }
  }
}

impl From<CheckedSensorReadCmdV4> for SensorReadCmdV4 {
  fn from(value: CheckedSensorReadCmdV4) -> Self {
    Self::new(
      value.device_index(),
      value.feature_index(),
      value.sensor_type(),
    )
  }
}

impl ButtplugMessageValidator for CheckedSensorReadCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
    // TODO Should expected_length always be > 0?
  }
}

impl TryFromDeviceAttributes<SensorReadCmdV4> for CheckedSensorReadCmdV4 {
  fn try_from_device_attributes(
    msg: SensorReadCmdV4,
    features: &crate::server::message::ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    if let Some(feature) = features.features().get(*msg.feature_index() as usize) {
      if feature.sensor().is_some() {
        Ok(CheckedSensorReadCmdV4::new(
          msg.device_index(),
          *msg.feature_index(),
          *msg.sensor_type(),
          feature.id(),
        ))
      } else {
        Err(ButtplugError::from(
          ButtplugDeviceError::DeviceNoSensorError("SensorReadCmd".to_string()),
        ))
      }
    } else {
      Err(ButtplugError::from(
        ButtplugDeviceError::DeviceFeatureIndexError(
          features.features().len() as u32,
          *msg.feature_index(),
        ),
      ))
    }
  }
}
