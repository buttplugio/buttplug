// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::TryFromDeviceAttributes;
use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
  message::{
    ButtplugDeviceMessage,
    ButtplugMessage,
    ButtplugMessageFinalizer,
    ButtplugMessageValidator,
    InputCmdV4,
    InputCommandType,
    InputType,
  },
};
use getset::CopyGetters;
use uuid::Uuid;

#[derive(
  Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone, CopyGetters,
)]
#[getset(get_copy = "pub")]
pub struct CheckedInputCmdV4 {
  id: u32,
  device_index: u32,
  feature_index: u32,
  input_type: InputType,
  input_command: InputCommandType,
  feature_id: Uuid,
}

impl CheckedInputCmdV4 {
  pub fn new(
    device_index: u32,
    feature_index: u32,
    input_type: InputType,
    input_command: InputCommandType,
    feature_id: Uuid,
  ) -> Self {
    Self {
      id: 1,
      device_index,
      feature_index,
      input_type,
      input_command,
      feature_id,
    }
  }
}

impl ButtplugMessageValidator for CheckedInputCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
    // TODO Should expected_length always be > 0?
  }
}

impl TryFromDeviceAttributes<InputCmdV4> for CheckedInputCmdV4 {
  fn try_from_device_attributes(
    msg: InputCmdV4,
    features: &crate::message::ServerDeviceAttributes,
  ) -> Result<Self, buttplug_core::errors::ButtplugError> {
    if let Some(feature) = features.features().get(msg.feature_index() as usize) {
      if let Some(sensor_map) = feature.input() {
        if let Some(sensor) = sensor_map.get(&msg.input_type()) {
          if sensor.input_commands().contains(&msg.input_command()) {
            Ok(CheckedInputCmdV4::new(
              msg.device_index(),
              msg.feature_index(),
              msg.input_type(),
              msg.input_command(),
              feature.id(),
            ))
          } else {
            Err(ButtplugError::from(
              ButtplugDeviceError::DeviceNoSensorError("InputCmd".to_string()),
            ))
          }
        } else {
          Err(ButtplugError::from(
            ButtplugDeviceError::DeviceNoSensorError("InputCmd".to_string()),
          ))
        }
      } else {
        Err(ButtplugError::from(
          ButtplugDeviceError::DeviceNoSensorError("InputCmd".to_string()),
        ))
      }
    } else {
      Err(ButtplugError::from(
        ButtplugDeviceError::DeviceFeatureIndexError(
          features.features().len() as u32,
          msg.feature_index(),
        ),
      ))
    }
  }
}
