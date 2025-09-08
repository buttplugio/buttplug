use buttplug_core::message::OutputType;

pub enum ClientDeviceCommandValue {
  Int(u32),
  Float(f64),
}

impl From<u32> for ClientDeviceCommandValue {
  fn from(val: u32) -> Self {
    ClientDeviceCommandValue::Int(val)
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
  Rotate(u32),
  Oscillate(u32),
  Constrict(u32),
  Heater(u32),
  Led(u32),
  Spray(u32),
  Position(u32),
  RotateWithDirection(u32, bool),
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
  RotateWithDirectionFloat(f64, bool),
  PositionWithDurationFloat(f64, u32),
}

impl From<&ClientDeviceOutputCommand> for OutputType {
  fn from(val: &ClientDeviceOutputCommand) -> Self {
    match val {
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
      ClientDeviceOutputCommand::RotateWithDirection(_, _)
      | ClientDeviceOutputCommand::RotateWithDirectionFloat(_, _) => {
        OutputType::RotateWithDirection
      }
    }
  }
}
