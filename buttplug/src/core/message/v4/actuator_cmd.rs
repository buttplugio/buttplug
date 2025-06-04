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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActuatorValue {
  value: u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActuatorPositionWithDuration {
  position: u32,
  duration: u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActuatorRotateWithDirection {
  speed: u32,
  clockwise: bool
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActuatorCommand {
  Vibrate(ActuatorValue),
  // Single Direction Rotation Speed
  Rotate(ActuatorValue),
  // Two Direction Rotation Speed
  RotateWithDirection(ActuatorRotateWithDirection),
  Oscillate(ActuatorValue),
  Constrict(ActuatorValue),
  Inflate(ActuatorValue),
  Heater(ActuatorValue),
  Led(ActuatorValue),
  // For instances where we specify a position to move to ASAP. Usually servos, probably for the
  // OSR-2/SR-6.
  Position(ActuatorValue),
  PositionWithDuration(ActuatorPositionWithDuration),
}

#[derive(
  Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, CopyGetters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
#[getset(get_copy="pub")]
pub struct ActuatorCmdV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "FeatureIndex"))]
  feature_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ActuatorType"))]
  actuator_type: ActuatorType,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Value"))]
  command: ActuatorCommand,
}

impl ActuatorCmdV4 {
  pub fn new(device_index: u32, feature_index: u32, actuator_type: ActuatorType, command: ActuatorCommand) -> Self {
    Self {
      id: 1,
      device_index,
      feature_index,
      actuator_type,
      command,
    }
  }
}

impl ButtplugMessageValidator for ActuatorCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    Ok(())
  }
}
