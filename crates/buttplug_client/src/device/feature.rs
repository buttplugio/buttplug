use std::sync::Arc;

use futures::{future, FutureExt};
use getset::{CopyGetters, Getters};

use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
  message::{
    ButtplugDeviceMessageNameV4, ButtplugServerMessageV4, DeviceFeature, InputCmdV4, InputCommandType, InputType, InputTypeData, OutputCmdV4, OutputCommand, OutputPositionWithDuration, OutputRotateWithDirection, OutputType, OutputValue
  },
};

use super::ClientDeviceOutputCommand;

use crate::{
  create_boxed_future_client_error,
  ButtplugClientError,
  ButtplugClientMessageSender,
  ButtplugClientResultFuture,
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

  pub fn check_and_set_actuator_value_float(
    &self,
    actuator_type: OutputType,
    value: f64,
  ) -> ButtplugClientResultFuture {
    if let Some(output_map) = self.feature().output() {
      if let Some(actuator) = output_map.get(&actuator_type) {
        self.event_loop_sender.send_message_expect_ok(
          OutputCmdV4::new(
            self.device_index,
            self.feature_index,
            OutputCommand::from_output_type(
              actuator_type,
              (value * actuator.step_count() as f64).ceil() as u32,
            )
            .unwrap(),
          )
          .into(),
        )
      } else {
        future::ready(Err(ButtplugClientError::from(ButtplugError::from(
          ButtplugDeviceError::DeviceActuatorTypeMismatch(
            self.feature_index,
            actuator_type,
            self.feature.feature_type(),
          ),
        ))))
        .boxed()
      }
    } else {
      future::ready(Err(ButtplugClientError::from(ButtplugError::from(
        ButtplugDeviceError::DeviceActuatorTypeMismatch(
          self.feature_index,
          actuator_type,
          self.feature.feature_type(),
        ),
      ))))
      .boxed()
    }
  }

  pub fn check_and_set_actuator(
    &self,
    output_command: OutputCommand,
  ) -> ButtplugClientResultFuture {
    let actuator_type = output_command.as_output_type();
    if let Some(output_map) = self.feature().output() {
      if output_map.get(&actuator_type).is_some() {
        self.event_loop_sender.send_message_expect_ok(
          OutputCmdV4::new(self.device_index, self.feature_index, output_command).into(),
        )
      } else {
        future::ready(Err(ButtplugClientError::from(ButtplugError::from(
          ButtplugDeviceError::DeviceActuatorTypeMismatch(
            self.feature_index,
            actuator_type,
            self.feature.feature_type(),
          ),
        ))))
        .boxed()
      }
    } else {
      future::ready(Err(ButtplugClientError::from(ButtplugError::from(
        ButtplugDeviceError::DeviceActuatorTypeMismatch(
          self.feature_index,
          actuator_type,
          self.feature.feature_type(),
        ),
      ))))
      .boxed()
    }
  }

  pub fn send_command(&self, client_device_command: &ClientDeviceOutputCommand) -> ButtplugClientResultFuture {
    match client_device_command.to_output_command(&self.feature) {
      Ok(cmd) => self.event_loop_sender.send_message_expect_ok(
        OutputCmdV4::new(self.device_index, self.feature_index, cmd).into()
      ),
      Err(e) => future::ready(Err(e)).boxed()
    }
  }

  pub fn vibrate(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_actuator(OutputCommand::Vibrate(OutputValue::new(level)))
  }

  pub fn oscillate(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_actuator(OutputCommand::Oscillate(OutputValue::new(level)))
  }

  pub fn rotate(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_actuator(OutputCommand::Rotate(OutputValue::new(level)))
  }

  pub fn spray(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_actuator(OutputCommand::Spray(OutputValue::new(level)))
  }

  pub fn constrict(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_actuator(OutputCommand::Constrict(OutputValue::new(level)))
  }

  pub fn position(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_actuator(OutputCommand::Position(OutputValue::new(level)))
  }

  pub fn position_with_duration(
    &self,
    position: u32,
    duration_in_ms: u32,
  ) -> ButtplugClientResultFuture {
    self.check_and_set_actuator(OutputCommand::PositionWithDuration(
      OutputPositionWithDuration::new(position, duration_in_ms),
    ))
  }

  pub fn rotate_with_direction(&self, level: u32, clockwise: bool) -> ButtplugClientResultFuture {
    self.check_and_set_actuator(OutputCommand::RotateWithDirection(
      OutputRotateWithDirection::new(level, clockwise),
    ))
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
