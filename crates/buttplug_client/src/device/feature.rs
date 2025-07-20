use std::sync::Arc;

use futures::{future, FutureExt};
use getset::{CopyGetters, Getters};

use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
  message::{
    ButtplugDeviceMessageNameV4, ButtplugServerMessageV4, DeviceFeature, DeviceFeatureOutput, InputCmdV4, InputCommandType, InputType, InputTypeData, OutputCmdV4, OutputCommand, OutputPositionWithDuration, OutputRotateWithDirection, OutputType, OutputValue
  },
};

use super::ClientDeviceOutputCommand;

use crate::{
  create_boxed_future_client_error, device::ClientDeviceCommandValue, ButtplugClientError, ButtplugClientMessageSender, ButtplugClientResultFuture
};

#[derive(Getters, CopyGetters, Clone)]
pub struct ClientDeviceFeature {
  #[getset(get_copy = "pub")]
  device_index: u32,
  #[getset(get_copy = "pub")]
  feature_index: u32,
  #[getset(get = "pub")]
  feature: DeviceFeature,
  /// Sends commands from the [ButtplugClientDevice] instance to the
  /// [ButtplugClient][super::ButtplugClient]'s event loop, which will then send
  /// the message on to the [ButtplugServer][crate::server::ButtplugServer]
  /// through the connector.
  event_loop_sender: Arc<ButtplugClientMessageSender>,
}

impl ClientDeviceFeature {
  pub(super) fn new(
    device_index: u32,
    feature_index: u32,
    feature: &DeviceFeature,
    event_loop_sender: &Arc<ButtplugClientMessageSender>,
  ) -> Self {
    Self {
      device_index,
      feature_index,
      feature: feature.clone(),
      event_loop_sender: event_loop_sender.clone(),
    }
  }

  fn check_step_value(&self, feature_output: &DeviceFeatureOutput, steps: u32) -> Result<u32, ButtplugClientError> {
    if steps <= feature_output.step_count() {
      Ok(steps)
    } else {
      Err(ButtplugClientError::ButtplugOutputCommandConversionError(format!("{} is larger than the maximum number of steps ({}).", steps, feature_output.step_count())))
    }
  }

  fn convert_float_value(&self, feature_output: &DeviceFeatureOutput, float_amt: f64) -> Result<u32, ButtplugClientError> {
    if float_amt < 0.0f64 || float_amt > 1.0f64 {
      Err(ButtplugClientError::ButtplugOutputCommandConversionError("Float values must be between 0.0 and 1.0".to_owned()))
    } else {
      Ok((float_amt * feature_output.step_count() as f64).ceil() as u32)
    }
  }

  pub(super) fn convert_client_cmd_to_output_cmd(&self, client_cmd: &ClientDeviceOutputCommand) -> Result<OutputCmdV4, ButtplugClientError> {
    let output_type: OutputType = client_cmd.into(); 
    // First off, make sure we support this output.
    let output = self
      .feature
      .output()
      .as_ref()
      .ok_or(ButtplugClientError::ButtplugOutputCommandConversionError(format!("Device feature does not support output type {}", output_type)))?
      .get(&output_type)
      .ok_or(ButtplugClientError::ButtplugOutputCommandConversionError(format!("Device feature does not support output type {}", output_type)))?;      

    let output_cmd = match client_cmd {
      ClientDeviceOutputCommand::VibrateFloat(v) => OutputCommand::Vibrate(OutputValue::new(self.convert_float_value(output, *v)?)),
      ClientDeviceOutputCommand::OscillateFloat(v) => OutputCommand::Oscillate(OutputValue::new(self.convert_float_value(output, *v)?)),
      ClientDeviceOutputCommand::RotateFloat(v) => OutputCommand::Rotate(OutputValue::new(self.convert_float_value(output, *v)?)),
      ClientDeviceOutputCommand::ConstrictFloat(v) => OutputCommand::Constrict(OutputValue::new(self.convert_float_value(output, *v)?)),
      ClientDeviceOutputCommand::HeaterFloat(v) => OutputCommand::Heater(OutputValue::new(self.convert_float_value(output, *v)?)),
      ClientDeviceOutputCommand::LedFloat(v )=> OutputCommand::Led(OutputValue::new(self.convert_float_value(output, *v)?)),
      ClientDeviceOutputCommand::SprayFloat(v) => OutputCommand::Spray(OutputValue::new(self.convert_float_value(output, *v)?)),
      ClientDeviceOutputCommand::PositionFloat(v) => OutputCommand::Position(OutputValue::new(self.convert_float_value(output, *v)?)),
      ClientDeviceOutputCommand::PositionWithDurationFloat(v, d) => OutputCommand::PositionWithDuration(OutputPositionWithDuration::new(self.convert_float_value(output, *v)?, *d)),
      ClientDeviceOutputCommand::RotateWithDirectionFloat(v, d) => OutputCommand::RotateWithDirection(OutputRotateWithDirection::new(self.convert_float_value(output, *v)?, *d)),
      ClientDeviceOutputCommand::Vibrate(v) => OutputCommand::Vibrate(OutputValue::new(self.check_step_value(output, *v)?)),
      ClientDeviceOutputCommand::Oscillate(v) => OutputCommand::Oscillate(OutputValue::new(self.check_step_value(output, *v)?)),
      ClientDeviceOutputCommand::Rotate(v) => OutputCommand::Rotate(OutputValue::new(self.check_step_value(output, *v)?)),
      ClientDeviceOutputCommand::Constrict(v) => OutputCommand::Constrict(OutputValue::new(self.check_step_value(output, *v)?)),
      ClientDeviceOutputCommand::Heater(v) => OutputCommand::Heater(OutputValue::new(self.check_step_value(output, *v)?)),
      ClientDeviceOutputCommand::Led(v )=> OutputCommand::Led(OutputValue::new(self.check_step_value(output, *v)?)),
      ClientDeviceOutputCommand::Spray(v) => OutputCommand::Spray(OutputValue::new(self.check_step_value(output, *v)?)),
      ClientDeviceOutputCommand::Position(v) => OutputCommand::Position(OutputValue::new(self.check_step_value(output, *v)?)),
      ClientDeviceOutputCommand::PositionWithDuration(v, d) => OutputCommand::PositionWithDuration(OutputPositionWithDuration::new(self.check_step_value(output, *v)?, *d)),
      ClientDeviceOutputCommand::RotateWithDirection(v, d) => OutputCommand::RotateWithDirection(OutputRotateWithDirection::new(self.check_step_value(output, *v)?, *d)),
    };
    Ok(OutputCmdV4::new(self.device_index, self.feature_index, output_cmd))
  }

  pub fn send_command(&self, client_device_command: &ClientDeviceOutputCommand) -> ButtplugClientResultFuture {
    match self.convert_client_cmd_to_output_cmd(&client_device_command) {
      Ok(cmd) => self.event_loop_sender.send_message_expect_ok(cmd.into()),
      Err(e) => future::ready(Err(e)).boxed()
    }
  }

  pub fn vibrate(&self, level: impl Into<ClientDeviceCommandValue>) -> ButtplugClientResultFuture {
    let val = level.into();
    self.send_command(&match val {
      ClientDeviceCommandValue::Int(v) => ClientDeviceOutputCommand::Vibrate(v),
      ClientDeviceCommandValue::Float(f) => ClientDeviceOutputCommand::VibrateFloat(f)
    })
  }

  pub fn oscillate(&self, level: impl Into<ClientDeviceCommandValue>) -> ButtplugClientResultFuture {
    let val = level.into();
    self.send_command(&match val {
      ClientDeviceCommandValue::Int(v) => ClientDeviceOutputCommand::Oscillate(v),
      ClientDeviceCommandValue::Float(f) => ClientDeviceOutputCommand::OscillateFloat(f)
    })
  }

  pub fn rotate(&self, level: impl Into<ClientDeviceCommandValue>) -> ButtplugClientResultFuture {
    let val = level.into();
    self.send_command(&match val {
      ClientDeviceCommandValue::Int(v) => ClientDeviceOutputCommand::Rotate(v),
      ClientDeviceCommandValue::Float(f) => ClientDeviceOutputCommand::RotateFloat(f)
    })
  }

  pub fn spray(&self, level: impl Into<ClientDeviceCommandValue>) -> ButtplugClientResultFuture {
    let val = level.into();
    self.send_command(&match val {
      ClientDeviceCommandValue::Int(v) => ClientDeviceOutputCommand::Spray(v),
      ClientDeviceCommandValue::Float(f) => ClientDeviceOutputCommand::SprayFloat(f)
    })
  }

  pub fn constrict(&self, level: impl Into<ClientDeviceCommandValue>) -> ButtplugClientResultFuture {
    let val = level.into();
    self.send_command(&match val {
      ClientDeviceCommandValue::Int(v) => ClientDeviceOutputCommand::Constrict(v),
      ClientDeviceCommandValue::Float(f) => ClientDeviceOutputCommand::ConstrictFloat(f)
    })
  }

  pub fn position(&self, level: impl Into<ClientDeviceCommandValue>) -> ButtplugClientResultFuture {
    let val = level.into();
    self.send_command(&match val {
      ClientDeviceCommandValue::Int(v) => ClientDeviceOutputCommand::Position(v),
      ClientDeviceCommandValue::Float(f) => ClientDeviceOutputCommand::PositionFloat(f)
    })
  }

  pub fn position_with_duration(
    &self,
    position: impl Into<ClientDeviceCommandValue>,
    duration_in_ms: u32,
  ) -> ButtplugClientResultFuture {
    let val = position.into();
    self.send_command(&match val {
      ClientDeviceCommandValue::Int(v) => ClientDeviceOutputCommand::PositionWithDuration(v, duration_in_ms),
      ClientDeviceCommandValue::Float(f) => ClientDeviceOutputCommand::PositionWithDurationFloat(f, duration_in_ms)
    })
  }

  pub fn rotate_with_direction(&self, level: impl Into<ClientDeviceCommandValue>, clockwise: bool) -> ButtplugClientResultFuture {
    let val = level.into();
    self.send_command(&match val {
      ClientDeviceCommandValue::Int(v) => ClientDeviceOutputCommand::RotateWithDirection(v, clockwise),
      ClientDeviceCommandValue::Float(f) => ClientDeviceOutputCommand::RotateWithDirectionFloat(f, clockwise)
    })
  }

  pub fn subscribe_sensor(&self, sensor_type: InputType) -> ButtplugClientResultFuture {
    if let Some(sensor_map) = self.feature.input() {
      if let Some(sensor) = sensor_map.get(&sensor_type) {
        if sensor
          .input_commands()
          .contains(&InputCommandType::Subscribe)
        {
          let msg = InputCmdV4::new(
            self.device_index,
            self.feature_index,
            sensor_type,
            InputCommandType::Subscribe,
          )
          .into();
          return self.event_loop_sender.send_message_expect_ok(msg);
        }
      }
    }
    create_boxed_future_client_error(
      ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageNameV4::InputCmd.to_string())
        .into(),
    )
  }

  pub fn unsubscribe_sensor(&self, sensor_type: InputType) -> ButtplugClientResultFuture {
    if let Some(sensor_map) = self.feature.input() {
      if let Some(sensor) = sensor_map.get(&sensor_type) {
        if sensor
          .input_commands()
          .contains(&InputCommandType::Subscribe)
        {
          let msg = InputCmdV4::new(
            self.device_index,
            self.feature_index,
            sensor_type,
            InputCommandType::Unsubscribe,
          )
          .into();
          return self.event_loop_sender.send_message_expect_ok(msg);
        }
      }
    }
    create_boxed_future_client_error(
      ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageNameV4::InputCmd.to_string())
        .into(),
    )
  }

  fn read_sensor(&self, sensor_type: InputType) -> ButtplugClientResultFuture<InputTypeData> {
    if let Some(sensor_map) = self.feature.input() {
      if let Some(sensor) = sensor_map.get(&sensor_type) {
        if sensor.input_commands().contains(&InputCommandType::Read) {
          let msg = InputCmdV4::new(
            self.device_index,
            self.feature_index,
            sensor_type,
            InputCommandType::Read,
          )
          .into();
          let reply = self.event_loop_sender.send_message(msg);
          return async move {
            if let ButtplugServerMessageV4::InputReading(data) = reply.await? {
              if sensor_type == data.data().as_input_type() {
                Ok(data.data())
              } else {
                Err(ButtplugError::ButtplugMessageError(ButtplugMessageError::UnexpectedMessageType(
                  "InputReading".to_owned(),
                ))
                .into())
              }
            } else {
              Err(
                ButtplugError::ButtplugMessageError(ButtplugMessageError::UnexpectedMessageType(
                  "InputReading".to_owned(),
                ))
                .into()
              )
            }
          }
          .boxed();
        }
      }
    }
    create_boxed_future_client_error(
      ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageNameV4::InputCmd.to_string())
        .into(),
    )
  }

  pub fn battery_level(&self) -> ButtplugClientResultFuture<u32> {
    if self
      .feature()
      .input()
      .as_ref()
      .ok_or(false)
      .unwrap()
      .contains_key(&InputType::Battery)
    {
      let send_fut = self.read_sensor(InputType::Battery);
      Box::pin(async move {
        let data = send_fut.await?;
        let battery_level = if let InputTypeData::Battery(level) = data {
          level.data()
        } else {
          0
        };
        Ok(battery_level as u32)
      })
    } else {
      create_boxed_future_client_error(
        ButtplugDeviceError::DeviceFeatureMismatch("Device feature is not battery".to_owned())
          .into(),
      )
    }
  }

  pub fn rssi_level(&self) -> ButtplugClientResultFuture<i8> {
    if self
      .feature()
      .input()
      .as_ref()
      .ok_or(false)
      .unwrap()
      .contains_key(&InputType::Rssi)
    {
      let send_fut = self.read_sensor(InputType::Rssi);
      Box::pin(async move {
        let data = send_fut.await?;
        let rssi_level = if let InputTypeData::Rssi(level) = data {
          level.data()
        } else {
          0
        };
        Ok(rssi_level)
      })
    } else {
      create_boxed_future_client_error(
        ButtplugDeviceError::DeviceFeatureMismatch("Device feature is not RSSI".to_owned()).into(),
      )
    }
  }
}
