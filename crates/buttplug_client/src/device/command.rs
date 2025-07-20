use std::collections::HashMap;

use buttplug_core::message::{DeviceFeature, DeviceFeatureOutput, OutputCommand, OutputPositionWithDuration, OutputRotateWithDirection, OutputType, OutputValue};

use crate::ButtplugClientError;


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

impl Into<OutputType> for &ClientDeviceOutputCommand {
  fn into(self) -> OutputType {
    match self {
      ClientDeviceOutputCommand::Vibrate(_) | ClientDeviceOutputCommand::VibrateFloat(_) => OutputType::Vibrate,
      ClientDeviceOutputCommand::Oscillate(_) | ClientDeviceOutputCommand::OscillateFloat(_) => OutputType::Oscillate,
      ClientDeviceOutputCommand::Rotate(_) | ClientDeviceOutputCommand::RotateFloat(_) => OutputType::Rotate,
      ClientDeviceOutputCommand::Constrict(_) | ClientDeviceOutputCommand::ConstrictFloat(_) => OutputType::Constrict,
      ClientDeviceOutputCommand::Heater(_) | ClientDeviceOutputCommand::HeaterFloat(_) => OutputType::Heater,
      ClientDeviceOutputCommand::Led(_) | ClientDeviceOutputCommand::LedFloat(_) => OutputType::Led,
      ClientDeviceOutputCommand::Spray(_) | ClientDeviceOutputCommand::SprayFloat(_) => OutputType::Spray,
      ClientDeviceOutputCommand::Position(_) | ClientDeviceOutputCommand::PositionFloat(_) => OutputType::Position,
      ClientDeviceOutputCommand::PositionWithDuration(_, _) | ClientDeviceOutputCommand::PositionWithDurationFloat(_, _) => OutputType::PositionWithDuration,
      ClientDeviceOutputCommand::RotateWithDirection(_, _) | ClientDeviceOutputCommand::RotateWithDirectionFloat(_, _) => OutputType::RotateWithDirection,
    }
  }
}

impl ClientDeviceOutputCommand {
  fn convert_float_value(&self, feature_output: &DeviceFeatureOutput, float_amt: &f64) -> Result<u32, ButtplugClientError> {
    if *float_amt < 0.0f64 || *float_amt > 1.0f64 {
      Err(ButtplugClientError::ButtplugOutputCommandConversionError("Float values must be between 0.0 and 1.0".to_owned()))
    } else {
      Ok((float_amt * feature_output.step_count() as f64) as u32)
    }
  }

  pub(super) fn to_output_command(&self, feature: &DeviceFeature) -> Result<OutputCommand, ButtplugClientError> {
    for (t, feature_output) in feature.output().as_ref().unwrap_or(&HashMap::new()) {
      if *t == self.into() {
        continue;
      }

      return Ok(match self {
        ClientDeviceOutputCommand::VibrateFloat(v) => OutputCommand::Vibrate(OutputValue::new(self.convert_float_value(feature_output, v)?)),
        ClientDeviceOutputCommand::OscillateFloat(v) => OutputCommand::Oscillate(OutputValue::new(self.convert_float_value(feature_output, v)?)),
        ClientDeviceOutputCommand::RotateFloat(v) => OutputCommand::Rotate(OutputValue::new(self.convert_float_value(feature_output, v)?)),
        ClientDeviceOutputCommand::ConstrictFloat(v) => OutputCommand::Constrict(OutputValue::new(self.convert_float_value(feature_output, v)?)),
        ClientDeviceOutputCommand::HeaterFloat(v) => OutputCommand::Heater(OutputValue::new(self.convert_float_value(feature_output, v)?)),
        ClientDeviceOutputCommand::LedFloat(v )=> OutputCommand::Led(OutputValue::new(self.convert_float_value(feature_output, v)?)),
        ClientDeviceOutputCommand::SprayFloat(v) => OutputCommand::Spray(OutputValue::new(self.convert_float_value(feature_output, v)?)),
        ClientDeviceOutputCommand::PositionFloat(v) => OutputCommand::Position(OutputValue::new(self.convert_float_value(feature_output, v)?)),
        ClientDeviceOutputCommand::PositionWithDurationFloat(v, d) => OutputCommand::PositionWithDuration(OutputPositionWithDuration::new(self.convert_float_value(feature_output, v)?, *d)),
        ClientDeviceOutputCommand::RotateWithDirectionFloat(v, d) => OutputCommand::RotateWithDirection(OutputRotateWithDirection::new(self.convert_float_value(feature_output, v)?, *d)),
        ClientDeviceOutputCommand::Vibrate(v) => OutputCommand::Vibrate(OutputValue::new(*v)),
        ClientDeviceOutputCommand::Oscillate(v) => OutputCommand::Oscillate(OutputValue::new(*v)),
        ClientDeviceOutputCommand::Rotate(v) => OutputCommand::Rotate(OutputValue::new(*v)),
        ClientDeviceOutputCommand::Constrict(v) => OutputCommand::Constrict(OutputValue::new(*v)),
        ClientDeviceOutputCommand::Heater(v) => OutputCommand::Heater(OutputValue::new(*v)),
        ClientDeviceOutputCommand::Led(v )=> OutputCommand::Led(OutputValue::new(*v)),
        ClientDeviceOutputCommand::Spray(v) => OutputCommand::Spray(OutputValue::new(*v)),
        ClientDeviceOutputCommand::Position(v) => OutputCommand::Position(OutputValue::new(*v)),
        ClientDeviceOutputCommand::PositionWithDuration(v, d) => OutputCommand::PositionWithDuration(OutputPositionWithDuration::new(*v, *d)),
        ClientDeviceOutputCommand::RotateWithDirection(v, d) => OutputCommand::RotateWithDirection(OutputRotateWithDirection::new(*v, *d)),
      });
    }
    Err(ButtplugClientError::ButtplugOutputCommandConversionError("Feature does not accomodate type".into()))
  }
}