use buttplug_core::message::OutputType;

use crate::ButtplugClientError;

#[derive(Debug, Clone, Copy)]
pub enum ClientDeviceCommandValue {
  Steps(i32),
  Percent(f64),
}

impl From<i32> for ClientDeviceCommandValue {
  fn from(val: i32) -> Self {
    ClientDeviceCommandValue::Steps(val)
  }
}

impl From<u32> for ClientDeviceCommandValue {
  fn from(val: u32) -> Self {
    ClientDeviceCommandValue::Steps(val as i32)
  }
}

impl From<f64> for ClientDeviceCommandValue {
  fn from(val: f64) -> Self {
    ClientDeviceCommandValue::Percent(val)
  }
}

pub enum ClientDeviceOutputCommand {
  // u32 types use steps, need to compare before sending
  Vibrate(ClientDeviceCommandValue),
  Rotate(ClientDeviceCommandValue),
  Oscillate(ClientDeviceCommandValue),
  Constrict(ClientDeviceCommandValue),
  Temperature(ClientDeviceCommandValue),
  Led(ClientDeviceCommandValue),
  Spray(ClientDeviceCommandValue),
  Position(ClientDeviceCommandValue),
  HwPositionWithDuration(ClientDeviceCommandValue, u32),
}

impl ClientDeviceOutputCommand {
  pub fn from_command_value(
    output_type: OutputType,
    value: &ClientDeviceCommandValue,
  ) -> Result<Self, ButtplugClientError> {
    match output_type {
      OutputType::Vibrate => Ok(ClientDeviceOutputCommand::Vibrate(*value)),
      OutputType::Oscillate => Ok(ClientDeviceOutputCommand::Oscillate(*value)),
      OutputType::Rotate => Ok(ClientDeviceOutputCommand::Rotate(*value)),
      OutputType::Constrict => Ok(ClientDeviceOutputCommand::Constrict(*value)),
      OutputType::Temperature => Ok(ClientDeviceOutputCommand::Temperature(*value)),
      OutputType::Led => Ok(ClientDeviceOutputCommand::Led(*value)),
      OutputType::Spray => Ok(ClientDeviceOutputCommand::Spray(*value)),
      OutputType::Position => Ok(ClientDeviceOutputCommand::Position(*value)),
      _ => Err(ButtplugClientError::ButtplugOutputCommandConversionError(
        "Cannot use HwPositionWithDuration with this method".to_owned(),
      )),
    }
  }
}

impl From<&ClientDeviceOutputCommand> for OutputType {
  fn from(val: &ClientDeviceOutputCommand) -> Self {
    match val {
      ClientDeviceOutputCommand::Vibrate(_) => OutputType::Vibrate,
      ClientDeviceOutputCommand::Oscillate(_) => OutputType::Oscillate,
      ClientDeviceOutputCommand::Rotate(_) => OutputType::Rotate,
      ClientDeviceOutputCommand::Constrict(_) => OutputType::Constrict,
      ClientDeviceOutputCommand::Temperature(_) => OutputType::Temperature,
      ClientDeviceOutputCommand::Led(_) => OutputType::Led,
      ClientDeviceOutputCommand::Spray(_) => OutputType::Spray,
      ClientDeviceOutputCommand::Position(_) => OutputType::Position,
      ClientDeviceOutputCommand::HwPositionWithDuration(_, _) => OutputType::HwPositionWithDuration,
    }
  }
}
