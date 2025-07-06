// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  errors::{ButtplugDeviceError, ButtplugError},
  message::{
    ButtplugDeviceMessage,
    ButtplugMessage,
    ButtplugMessageError,
    ButtplugMessageFinalizer,
    ButtplugMessageValidator,
    OutputType,
  },
};
use getset::CopyGetters;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct OutputValue {
  #[serde(rename = "Value")]
  value: u32,
}

impl OutputValue {
  pub fn new(value: u32) -> Self {
    Self { value }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct OutputPositionWithDuration {
  #[serde(rename = "Position")]
  position: u32,
  #[serde(rename = "Duration")]
  duration: u32,
}

impl OutputPositionWithDuration {
  pub fn new(position: u32, duration: u32) -> Self {
    Self { position, duration }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct OutputRotateWithDirection {
  #[serde(rename = "Speed")]
  speed: u32,
  #[serde(rename = "Clockwise")]
  clockwise: bool,
}

impl OutputRotateWithDirection {
  pub fn new(speed: u32, clockwise: bool) -> Self {
    Self { speed, clockwise }
  }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputCommand {
  Vibrate(OutputValue),
  // Single Direction Rotation Speed
  Rotate(OutputValue),
  // Two Direction Rotation Speed
  RotateWithDirection(OutputRotateWithDirection),
  Oscillate(OutputValue),
  Constrict(OutputValue),
  Spray(OutputValue),
  Heater(OutputValue),
  Led(OutputValue),
  // For instances where we specify a position to move to ASAP. Usually servos, probably for the
  // OSR-2/SR-6.
  Position(OutputValue),
  PositionWithDuration(OutputPositionWithDuration),
}

impl OutputCommand {
  pub fn value(&self) -> u32 {
    match self {
      OutputCommand::Constrict(x)
      | OutputCommand::Spray(x)
      | OutputCommand::Heater(x)
      | OutputCommand::Led(x)
      | OutputCommand::Oscillate(x)
      | OutputCommand::Position(x)
      | OutputCommand::Rotate(x)
      | OutputCommand::Vibrate(x) => x.value(),
      OutputCommand::RotateWithDirection(x) => x.speed(),
      OutputCommand::PositionWithDuration(x) => x.position(),
    }
  }

  pub fn set_value(&mut self, value: u32) {
    match self {
      OutputCommand::Constrict(x)
      | OutputCommand::Spray(x)
      | OutputCommand::Heater(x)
      | OutputCommand::Led(x)
      | OutputCommand::Oscillate(x)
      | OutputCommand::Position(x)
      | OutputCommand::Rotate(x)
      | OutputCommand::Vibrate(x) => x.value = value,
      OutputCommand::RotateWithDirection(x) => x.speed = value,
      OutputCommand::PositionWithDuration(x) => x.position = value,
    }
  }

  pub fn as_output_type(&self) -> OutputType {
    match self {
      Self::Vibrate(_) => OutputType::Vibrate,
      Self::Rotate(_) => OutputType::Rotate,
      Self::RotateWithDirection(_) => OutputType::RotateWithDirection,
      Self::Oscillate(_) => OutputType::Oscillate,
      Self::Constrict(_) => OutputType::Constrict,
      Self::Spray(_) => OutputType::Spray,
      Self::Led(_) => OutputType::Led,
      Self::Position(_) => OutputType::Position,
      Self::PositionWithDuration(_) => OutputType::PositionWithDuration,
      Self::Heater(_) => OutputType::Heater,
    }
  }

  pub fn from_output_type(output_type: OutputType, value: u32) -> Result<Self, ButtplugError> {
    match output_type {
      OutputType::Constrict => Ok(Self::Constrict(OutputValue::new(value))),
      OutputType::Heater => Ok(Self::Heater(OutputValue::new(value))),
      OutputType::Spray => Ok(Self::Spray(OutputValue::new(value))),
      OutputType::Led => Ok(Self::Led(OutputValue::new(value))),
      OutputType::Oscillate => Ok(Self::Oscillate(OutputValue::new(value))),
      OutputType::Position => Ok(Self::Position(OutputValue::new(value))),
      OutputType::Rotate => Ok(Self::Rotate(OutputValue::new(value))),
      OutputType::Vibrate => Ok(Self::Vibrate(OutputValue::new(value))),
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
pub struct OutputCmdV4 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[serde(rename = "FeatureIndex")]
  feature_index: u32,
  #[serde(rename = "Command")]
  command: OutputCommand,
}

impl OutputCmdV4 {
  pub fn new(device_index: u32, feature_index: u32, command: OutputCommand) -> Self {
    Self {
      id: 1,
      device_index,
      feature_index,
      command,
    }
  }
}

impl ButtplugMessageValidator for OutputCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    Ok(())
  }
}
