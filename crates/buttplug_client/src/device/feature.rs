// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use futures::{FutureExt, future};
use getset::{CopyGetters, Getters};

use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
  message::{
    ButtplugDeviceMessageNameV4,
    ButtplugServerMessageV4,
    DeviceFeature,
    DeviceFeatureOutputLimits,
    InputCmdV4,
    InputCommandType,
    InputType,
    InputTypeReading,
    OutputCmdV4,
    OutputCommand,
    OutputHwPositionWithDuration,
    OutputType,
    OutputValue,
  },
};

use super::ClientDeviceOutputCommand;

use crate::{
  ButtplugClientError,
  ButtplugClientMessageSender,
  ButtplugClientResultFuture,
  create_boxed_future_client_error,
  device::ClientDeviceCommandValue,
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
  event_loop_sender: ButtplugClientMessageSender,
}

impl ClientDeviceFeature {
  pub(super) fn new(
    device_index: u32,
    feature_index: u32,
    feature: &DeviceFeature,
    event_loop_sender: &ButtplugClientMessageSender,
  ) -> Self {
    Self {
      device_index,
      feature_index,
      feature: feature.clone(),
      event_loop_sender: event_loop_sender.clone(),
    }
  }

  fn check_step_value(
    &self,
    feature_output: &dyn DeviceFeatureOutputLimits,
    steps: &ClientDeviceCommandValue,
  ) -> Result<i32, ButtplugClientError> {
    let value = match steps {
      ClientDeviceCommandValue::Percent(f) => self.convert_float_value(feature_output, *f)?,
      ClientDeviceCommandValue::Steps(i) => *i,
    };
    if feature_output.step_limit().contains(&value) {
      Ok(value)
    } else {
      Err(ButtplugClientError::ButtplugOutputCommandConversionError(
        format!(
          "{} is larger than the maximum number of steps ({}).",
          value,
          feature_output.step_count()
        ),
      ))
    }
  }

  fn convert_float_value(
    &self,
    feature_output: &dyn DeviceFeatureOutputLimits,
    float_amt: f64,
  ) -> Result<i32, ButtplugClientError> {
    if !(-1.0f64..=1.0f64).contains(&float_amt) {
      Err(ButtplugClientError::ButtplugOutputCommandConversionError(
        format!("Float values must be between 0.0 and 1.0, received value was {}", float_amt),
      ))
    } else {
      let mut val = float_amt * feature_output.step_count() as f64;
      val = if val > 0.000001f64 {
        val.ceil()
      } else {
        val.floor()
      };
      Ok(val as i32)
    }
  }

  pub(super) fn convert_client_cmd_to_output_cmd(
    &self,
    client_cmd: &ClientDeviceOutputCommand,
  ) -> Result<OutputCmdV4, ButtplugClientError> {
    let output_type: OutputType = client_cmd.into();
    // First off, make sure we support this output.
    let output = self
      .feature
      .output()
      .as_ref()
      .ok_or(ButtplugClientError::ButtplugOutputCommandConversionError(
        format!(
          "Device feature does not support output type {}",
          output_type
        ),
      ))?
      .get(output_type)
      .ok_or(ButtplugClientError::ButtplugOutputCommandConversionError(
        format!(
          "Device feature does not support output type {}",
          output_type
        ),
      ))?;

    let output_cmd = match client_cmd {
      ClientDeviceOutputCommand::Vibrate(v) => {
        OutputCommand::Vibrate(OutputValue::new(self.check_step_value(output, v)?))
      }
      ClientDeviceOutputCommand::Oscillate(v) => {
        OutputCommand::Oscillate(OutputValue::new(self.check_step_value(output, v)?))
      }
      ClientDeviceOutputCommand::Rotate(v) => {
        OutputCommand::Rotate(OutputValue::new(self.check_step_value(output, v)?))
      }
      ClientDeviceOutputCommand::Constrict(v) => {
        OutputCommand::Constrict(OutputValue::new(self.check_step_value(output, v)?))
      }
      ClientDeviceOutputCommand::Temperature(v) => {
        OutputCommand::Temperature(OutputValue::new(self.check_step_value(output, v)?))
      }
      ClientDeviceOutputCommand::Led(v) => {
        OutputCommand::Led(OutputValue::new(self.check_step_value(output, v)?))
      }
      ClientDeviceOutputCommand::Spray(v) => {
        OutputCommand::Spray(OutputValue::new(self.check_step_value(output, v)?))
      }
      ClientDeviceOutputCommand::Position(v) => {
        OutputCommand::Position(OutputValue::new(self.check_step_value(output, v)?))
      }
      ClientDeviceOutputCommand::HwPositionWithDuration(v, d) => OutputCommand::HwPositionWithDuration(
        OutputHwPositionWithDuration::new(self.check_step_value(output, v)? as u32, *d),
      ),
    };
    Ok(OutputCmdV4::new(
      self.device_index,
      self.feature_index,
      output_cmd,
    ))
  }

  pub fn run_output(
    &self,
    client_device_command: &ClientDeviceOutputCommand,
  ) -> ButtplugClientResultFuture {
    match self.convert_client_cmd_to_output_cmd(client_device_command) {
      Ok(cmd) => self.event_loop_sender.send_message_expect_ok(cmd.into()),
      Err(e) => future::ready(Err(e)).boxed(),
    }
  }

  pub fn run_input_subscribe(&self, sensor_type: InputType) -> ButtplugClientResultFuture {
    if let Some(sensor_map) = self.feature.input()
      && let Some(sensor) = sensor_map.get(sensor_type)
      && sensor.command().contains(&InputCommandType::Subscribe)
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
    create_boxed_future_client_error(
      ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageNameV4::InputCmd.to_string())
        .into(),
    )
  }

  pub fn run_input_unsubscribe(&self, sensor_type: InputType) -> ButtplugClientResultFuture {
    if let Some(sensor_map) = self.feature.input()
      && let Some(sensor) = sensor_map.get(sensor_type)
      && sensor.command().contains(&InputCommandType::Subscribe)
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
    create_boxed_future_client_error(
      ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageNameV4::InputCmd.to_string())
        .into(),
    )
  }

  pub fn run_input_read(&self, sensor_type: InputType) -> ButtplugClientResultFuture<InputTypeReading> {
    if let Some(sensor_map) = self.feature.input()
      && let Some(sensor) = sensor_map.get(sensor_type)
      && sensor.command().contains(&InputCommandType::Read)
    {
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
          if sensor_type == data.reading().into() {
            Ok(data.reading())
          } else {
            Err(
              ButtplugError::ButtplugMessageError(ButtplugMessageError::UnexpectedMessageType(
                "InputReading".to_owned(),
              ))
              .into(),
            )
          }
        } else {
          Err(
            ButtplugError::ButtplugMessageError(ButtplugMessageError::UnexpectedMessageType(
              "InputReading".to_owned(),
            ))
            .into(),
          )
        }
      }
      .boxed();
    }
    create_boxed_future_client_error(
      ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageNameV4::InputCmd.to_string())
        .into(),
    )
  }

  pub fn battery(&self) -> ButtplugClientResultFuture<u32> {
    if self
      .feature()
      .input()
      .as_ref()
      .ok_or(false)
      .unwrap()
      .contains(InputType::Battery)
    {
      let send_fut = self.run_input_read(InputType::Battery);
      Box::pin(async move {
        let data = send_fut.await?;
        let battery_level = if let InputTypeReading::Battery(level) = data {
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

  pub fn rssi(&self) -> ButtplugClientResultFuture<i8> {
    if self
      .feature()
      .input()
      .as_ref()
      .ok_or(false)
      .unwrap()
      .contains(InputType::Rssi)
    {
      let send_fut = self.run_input_read(InputType::Rssi);
      Box::pin(async move {
        let data = send_fut.await?;
        let rssi_level = if let InputTypeReading::Rssi(level) = data {
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
