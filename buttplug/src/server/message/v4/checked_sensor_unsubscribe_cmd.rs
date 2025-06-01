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
      ButtplugSensorFeatureMessageType,
      SensorType,
      SensorUnsubscribeCmdV4,
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
pub struct CheckedSensorUnsubscribeCmdV4 {
  id: u32,
  device_index: u32,
  feature_index: u32,
  sensor_type: SensorType,
  feature_id: Uuid,
}

impl CheckedSensorUnsubscribeCmdV4 {
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

impl From<CheckedSensorUnsubscribeCmdV4> for SensorUnsubscribeCmdV4 {
  fn from(value: CheckedSensorUnsubscribeCmdV4) -> Self {
    Self::new(
      value.device_index(),
      value.feature_index(),
      value.sensor_type(),
    )
  }
}

impl ButtplugMessageValidator for CheckedSensorUnsubscribeCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}

impl TryFromDeviceAttributes<SensorUnsubscribeCmdV4> for CheckedSensorUnsubscribeCmdV4 {
  fn try_from_device_attributes(
    msg: SensorUnsubscribeCmdV4,
    features: &crate::server::message::ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    if let Some(feature) = features.features().get(*msg.feature_index() as usize) {
      if let Some(sensor_map) = feature.sensor() {
        if let Some(sensor) = sensor_map.get(msg.sensor_type()) {
          if sensor
            .messages()
            .contains(&ButtplugSensorFeatureMessageType::SensorSubscribeCmd)
          {
            Ok(CheckedSensorUnsubscribeCmdV4::new(
              msg.device_index(),
              *msg.feature_index(),
              *msg.sensor_type(),
              feature.id(),
            ))
          } else {
            Err(ButtplugError::from(
              ButtplugDeviceError::MessageNotSupported("SensorUnsubscribeCmd".to_string()),
            ))
          }
        } else {
          Err(ButtplugError::from(
            ButtplugDeviceError::DeviceNoSensorError("SensorUnsubscribeCmd".to_string()),
          ))
        }
      } else {
        Err(ButtplugError::from(
          ButtplugDeviceError::DeviceNoSensorError("SensorUnsubscribeCmd".to_string()),
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
