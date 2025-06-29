// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  checked_input_cmd::CheckedInputCmdV4,
  ServerDeviceAttributes,
  TryFromDeviceAttributes,
};
use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
  message::{
    ButtplugDeviceMessage,
    ButtplugMessage,
    ButtplugMessageFinalizer,
    ButtplugMessageValidator,
    InputCommandType,
    InputType,
  },
};
use serde::{Deserialize, Serialize};

/// Battery level request
#[derive(
  Debug,
  ButtplugDeviceMessage,
  ButtplugMessageFinalizer,
  PartialEq,
  Eq,
  Clone,
  Serialize,
  Deserialize,
)]
pub struct BatteryLevelCmdV2 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
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

impl TryFromDeviceAttributes<BatteryLevelCmdV2> for CheckedInputCmdV4 {
  fn try_from_device_attributes(
    msg: BatteryLevelCmdV2,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, buttplug_core::errors::ButtplugError> {
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

    let feature_index = features
      .features()
      .iter()
      .enumerate()
      .find(|(_, p)| {
        if let Some(sensor_map) = p.input() {
          if sensor_map.contains_key(&InputType::Battery) {
            return true;
          }
        }
        false
      })
      .expect("Already found matching battery feature, can unwrap this.")
      .0;

    Ok(CheckedInputCmdV4::new(
      msg.device_index(),
      feature_index as u32,
      InputType::Battery,
      InputCommandType::Read,
      battery_feature.id(),
    ))
  }
}
