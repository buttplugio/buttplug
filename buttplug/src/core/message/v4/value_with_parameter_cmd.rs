// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::{
  ActuatorType, ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageError, ButtplugMessageFinalizer, ButtplugMessageValidator
};
use getset::CopyGetters;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
#[getset(get_copy="pub")]
pub struct ValueWithParameterCmdV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "FeatureIndex"))]
  feature_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ActuatorType"))]
  actuator_type: ActuatorType,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Duration"))]
  value: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Position"))]
  parameter: i32,
}

impl ValueWithParameterCmdV4 {
  pub fn new(device_index: u32, feature_index: u32, actuator_type: ActuatorType, value: u32, parameter: i32) -> Self {
    Self {
      id: 1,
      device_index,
      feature_index,
      actuator_type,
      value,
      parameter
    }
  }
}

impl ButtplugMessageValidator for ValueWithParameterCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    Ok(())
  }
}
