use buttplug_core::message::OutputType;

use crate::ButtplugClientError;

pub enum ClientDeviceCommandValue {
  Int(i32),
  Float(f64),
}

impl From<i32> for ClientDeviceCommandValue {
  fn from(val: i32) -> Self {
    ClientDeviceCommandValue::Int(val)
  }
}

impl From<u32> for ClientDeviceCommandValue {
  fn from(val: u32) -> Self {
    ClientDeviceCommandValue::Int(val as i32)
  }
}

impl From<f64> for ClientDeviceCommandValue {
  fn from(val: f64) -> Self {
    ClientDeviceCommandValue::Float(val)
  }
}

pub enum ClientDeviceOutputCommand {
  // u32 types use steps, need to compare before sending
  Vibrate(u32),
  Rotate(i32),
  Oscillate(u32),
  Constrict(u32),
  Heater(u32),
  Led(u32),
  Spray(u32),
  Position(u32),
  PositionWithDuration(u32, u32),
  // f64 types are old style float, will need to convert before sending
  VibrateFloat(f64),
  RotateFloat(f64),
  OscillateFloat(f64),
  ConstrictFloat(f64),
  HeaterFloat(f64),
  LedFloat(f64),
  SprayFloat(f64),
  PositionFloat(f64),
  PositionWithDurationFloat(f64, u32),
}

impl ClientDeviceOutputCommand {
  pub fn from_command_value_float(
    output_type: OutputType,
    value: f64,
  ) -> Result<Self, ButtplugClientError> {
    match output_type {
      OutputType::Vibrate => Ok(ClientDeviceOutputCommand::VibrateFloat(value)),
      OutputType::Oscillate => Ok(ClientDeviceOutputCommand::OscillateFloat(value)),
      OutputType::Rotate => Ok(ClientDeviceOutputCommand::RotateFloat(value)),
      OutputType::Constrict => Ok(ClientDeviceOutputCommand::ConstrictFloat(value)),
      OutputType::Heater => Ok(ClientDeviceOutputCommand::HeaterFloat(value)),
      OutputType::Led => Ok(ClientDeviceOutputCommand::LedFloat(value)),
      OutputType::Spray => Ok(ClientDeviceOutputCommand::SprayFloat(value)),
      OutputType::Position => Ok(ClientDeviceOutputCommand::PositionFloat(value)),
      _ => Err(ButtplugClientError::ButtplugOutputCommandConversionError(
        "Cannot use PositionWithDuration with this method".to_owned(),
      )),
    }
  }
}

impl Into<OutputType> for &ClientDeviceOutputCommand {
  fn into(self) -> OutputType {
    match self {
      ClientDeviceOutputCommand::Vibrate(_) | ClientDeviceOutputCommand::VibrateFloat(_) => {
        OutputType::Vibrate
      }
      ClientDeviceOutputCommand::Oscillate(_) | ClientDeviceOutputCommand::OscillateFloat(_) => {
        OutputType::Oscillate
      }
      ClientDeviceOutputCommand::Rotate(_) | ClientDeviceOutputCommand::RotateFloat(_) => {
        OutputType::Rotate
      }
      ClientDeviceOutputCommand::Constrict(_) | ClientDeviceOutputCommand::ConstrictFloat(_) => {
        OutputType::Constrict
      }
      ClientDeviceOutputCommand::Heater(_) | ClientDeviceOutputCommand::HeaterFloat(_) => {
        OutputType::Heater
      }
      ClientDeviceOutputCommand::Led(_) | ClientDeviceOutputCommand::LedFloat(_) => OutputType::Led,
      ClientDeviceOutputCommand::Spray(_) | ClientDeviceOutputCommand::SprayFloat(_) => {
        OutputType::Spray
      }
      ClientDeviceOutputCommand::Position(_) | ClientDeviceOutputCommand::PositionFloat(_) => {
        OutputType::Position
      }
      ClientDeviceOutputCommand::PositionWithDuration(_, _)
      | ClientDeviceOutputCommand::PositionWithDurationFloat(_, _) => {
        OutputType::PositionWithDuration
      }
    }
  }
}
