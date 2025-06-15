// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::{
  errors::{ButtplugDeviceError, ButtplugError},
  message::{
    ActuatorType,
    ButtplugDeviceMessage,
    ButtplugMessage,
    ButtplugMessageError,
    ButtplugMessageFinalizer,
    ButtplugMessageValidator,
  },
};
use getset::CopyGetters;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct ActuatorValue {
  #[serde(rename="Value")]
  value: u32,
}

impl ActuatorValue {
  pub fn new(value: u32) -> Self {
    Self { value }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct ActuatorPositionWithDuration {
  #[serde(rename="Position")]
  position: u32,
  #[serde(rename="Duration")]
  duration: u32,
}

impl ActuatorPositionWithDuration {
  pub fn new(position: u32, duration: u32) -> Self {
    Self { position, duration }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct ActuatorRotateWithDirection {
  #[serde(rename="Speed")]
  speed: u32,
  #[serde(rename="Clockwise")]
  clockwise: bool,
}

impl ActuatorRotateWithDirection {
  pub fn new(speed: u32, clockwise: bool) -> Self {
    Self { speed, clockwise }
  }
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

impl ActuatorCommand {
  pub fn value(&self) -> u32 {
    match self {
      ActuatorCommand::Constrict(x)
      | ActuatorCommand::Inflate(x)
      | ActuatorCommand::Heater(x)
      | ActuatorCommand::Led(x)
      | ActuatorCommand::Oscillate(x)
      | ActuatorCommand::Position(x)
      | ActuatorCommand::Rotate(x)
      | ActuatorCommand::Vibrate(x) => x.value(),
      ActuatorCommand::RotateWithDirection(x) => x.speed(),
      ActuatorCommand::PositionWithDuration(x) => x.position(),
    }
  }

  pub fn set_value(&mut self, value: u32) {
    match self {
      ActuatorCommand::Constrict(x)
      | ActuatorCommand::Inflate(x)
      | ActuatorCommand::Heater(x)
      | ActuatorCommand::Led(x)
      | ActuatorCommand::Oscillate(x)
      | ActuatorCommand::Position(x)
      | ActuatorCommand::Rotate(x)
      | ActuatorCommand::Vibrate(x) => x.value = value,
      ActuatorCommand::RotateWithDirection(x) => x.speed = value,
      ActuatorCommand::PositionWithDuration(x) => x.position = value,
    }
  }

  pub fn as_actuator_type(&self) -> ActuatorType {
    match self {
      Self::Vibrate(_) => ActuatorType::Vibrate,
      Self::Rotate(_) => ActuatorType::Rotate,
      Self::RotateWithDirection(_) => ActuatorType::RotateWithDirection,
      Self::Oscillate(_) => ActuatorType::Oscillate,
      Self::Constrict(_) => ActuatorType::Constrict,
      Self::Inflate(_) => ActuatorType::Inflate,
      Self::Led(_) => ActuatorType::Led,
      Self::Position(_) => ActuatorType::Position,
      Self::PositionWithDuration(_) => ActuatorType::PositionWithDuration,
      Self::Heater(_) => ActuatorType::Heater,
    }
  }

  pub fn from_actuator_type(
    actuator_type: ActuatorType,
    value: u32,
  ) -> Result<Self, ButtplugError> {
    match actuator_type {
      ActuatorType::Constrict => Ok(Self::Constrict(ActuatorValue::new(value))),
      ActuatorType::Heater => Ok(Self::Heater(ActuatorValue::new(value))),
      ActuatorType::Inflate => Ok(Self::Inflate(ActuatorValue::new(value))),
      ActuatorType::Led => Ok(Self::Led(ActuatorValue::new(value))),
      ActuatorType::Oscillate => Ok(Self::Oscillate(ActuatorValue::new(value))),
      ActuatorType::Position => Ok(Self::Position(ActuatorValue::new(value))),
      ActuatorType::Rotate => Ok(Self::Rotate(ActuatorValue::new(value))),
      ActuatorType::Vibrate => Ok(Self::Vibrate(ActuatorValue::new(value))),
      x => Err(ButtplugError::ButtplugDeviceError(
        ButtplugDeviceError::ActuatorNotSupported(x),
      )),
    }
  }
}

#[derive(
  Debug,
  ButtplugDeviceMessage,
  ButtplugMessageFinalizer,
  PartialEq,
  Clone,
  CopyGetters,
  Serialize,
  Deserialize,
)]
#[getset(get_copy = "pub")]
pub struct ActuatorCmdV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "FeatureIndex"))]
  feature_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Command"))]
  command: ActuatorCommand,
}

impl ActuatorCmdV4 {
  pub fn new(device_index: u32, feature_index: u32, command: ActuatorCommand) -> Self {
    Self {
      id: 1,
      device_index,
      feature_index,
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
