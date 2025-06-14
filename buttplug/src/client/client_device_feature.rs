use std::sync::Arc;

use futures::{future, FutureExt};
use getset::{CopyGetters, Getters};

use crate::{core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
    message::{
      ActuatorCmdV4, ActuatorCommand, ActuatorPositionWithDuration, ActuatorRotateWithDirection, ActuatorType, ActuatorValue, ButtplugServerMessageV4, DeviceFeature, SensorCmdV4, SensorCommandType, SensorType
    },
  }, server::message::spec_enums::ButtplugDeviceMessageNameV4};

use super::{
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
    actuator_type: ActuatorType,
    value: f64,
  ) -> ButtplugClientResultFuture {
    if let Some(actuator_map) = self.feature().actuator() {
      if let Some(actuator) = actuator_map.get(&actuator_type) {
        self.event_loop_sender.send_message_expect_ok(
          ActuatorCmdV4::new(self.device_index, self.feature_index, ActuatorCommand::from_actuator_type(actuator_type, (value * *actuator.step_count() as f64).ceil() as u32).unwrap()).into(),
        )  
      } else {
        future::ready(Err(ButtplugClientError::from(ButtplugError::from(
          ButtplugDeviceError::DeviceActuatorTypeMismatch(
            self.feature_index,
            actuator_type,
            *self.feature.feature_type(),
          ),
        ))))
        .boxed()
      }
    } else {
      future::ready(Err(ButtplugClientError::from(ButtplugError::from(
        ButtplugDeviceError::DeviceActuatorTypeMismatch(
          self.feature_index,
          actuator_type,
          *self.feature.feature_type(),
        ),
      ))))
      .boxed()
    }
  }

  pub fn check_and_set_actuator(
    &self,
    actuator_command: ActuatorCommand,
  ) -> ButtplugClientResultFuture {
    let actuator_type = actuator_command.as_actuator_type();
    if let Some(actuator_map) = self.feature().actuator() {
      if let Some(_) = actuator_map.get(&actuator_type) {
        self.event_loop_sender.send_message_expect_ok(
          ActuatorCmdV4::new(self.device_index, self.feature_index, actuator_command).into(),
        )        
      } else {
        future::ready(Err(ButtplugClientError::from(ButtplugError::from(
          ButtplugDeviceError::DeviceActuatorTypeMismatch(
            self.feature_index,
            actuator_type,
            *self.feature.feature_type(),
          ),
        ))))
        .boxed()
      }
    } else {
      future::ready(Err(ButtplugClientError::from(ButtplugError::from(
        ButtplugDeviceError::DeviceActuatorTypeMismatch(
          self.feature_index,
          actuator_type,
          *self.feature.feature_type(),
        ),
      ))))
      .boxed()
    }
  }

  pub fn vibrate(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_actuator(ActuatorCommand::Vibrate(ActuatorValue::new(level)))
  }

  pub fn oscillate(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_actuator(ActuatorCommand::Oscillate(ActuatorValue::new(level)))
  }

  pub fn rotate(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_actuator(ActuatorCommand::Rotate(ActuatorValue::new(level)))
  }

  pub fn inflate(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_actuator(ActuatorCommand::Inflate(ActuatorValue::new(level)))
  }

  pub fn constrict(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_actuator(ActuatorCommand::Constrict(ActuatorValue::new(level)))
  }

  pub fn position(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_actuator(ActuatorCommand::Position(ActuatorValue::new(level)))
  }

  pub fn position_with_duration(&self, position: u32, duration_in_ms: u32) -> ButtplugClientResultFuture {
    self.check_and_set_actuator(ActuatorCommand::PositionWithDuration(ActuatorPositionWithDuration::new(position, duration_in_ms)))
  }

  pub fn rotate_with_direction(&self, level: u32, clockwise: bool) -> ButtplugClientResultFuture {
    self.check_and_set_actuator(ActuatorCommand::RotateWithDirection(ActuatorRotateWithDirection::new(level, clockwise)))
  }

  pub fn subscribe_sensor(&self, sensor_type: SensorType) -> ButtplugClientResultFuture {
    if let Some(sensor_map) = self.feature.sensor() {
      if let Some(sensor) = sensor_map.get(&sensor_type) {
        if sensor
          .sensor_commands()
          .contains(&SensorCommandType::Subscribe)
        {
          let msg = SensorCmdV4::new(
            self.device_index,
            self.feature_index,
            sensor_type,
            SensorCommandType::Subscribe
          )
          .into();
          return self.event_loop_sender.send_message_expect_ok(msg)
        }
      }
    }
    create_boxed_future_client_error(
      ButtplugDeviceError::MessageNotSupported(
        ButtplugDeviceMessageNameV4::SensorCmd.to_string(),
      )
      .into(),
    )
  }
  
  pub fn unsubscribe_sensor(&self, sensor_type: SensorType) -> ButtplugClientResultFuture {
    if let Some(sensor_map) = self.feature.sensor() {
      if let Some(sensor) = sensor_map.get(&sensor_type) {
        if sensor
          .sensor_commands()
          .contains(&SensorCommandType::Subscribe)
        {
          let msg = SensorCmdV4::new(
            self.device_index,
            self.feature_index,
            sensor_type,
            SensorCommandType::Unsubscribe
          )
          .into();
          return self.event_loop_sender.send_message_expect_ok(msg);
        }
      }
    }
    create_boxed_future_client_error(
      ButtplugDeviceError::MessageNotSupported(
        ButtplugDeviceMessageNameV4::SensorCmd.to_string(),
      )
      .into())
  }

  fn read_sensor(&self, sensor_type: SensorType) -> ButtplugClientResultFuture<Vec<i32>> {
    if let Some(sensor_map) = self.feature.sensor() {
      if let Some(sensor) = sensor_map.get(&sensor_type) {
      if sensor
          .sensor_commands()
          .contains(&SensorCommandType::Read)
        {
        let msg = SensorCmdV4::new(
          self.device_index,
          self.feature_index,
          sensor_type,
          SensorCommandType::Read
        )
        .into();
        let reply = self.event_loop_sender.send_message(msg);
        return async move {
          if let ButtplugServerMessageV4::SensorReading(data) = reply.await? {
            Ok(data.data().clone())
          } else {
            Err(
              ButtplugError::ButtplugMessageError(ButtplugMessageError::UnexpectedMessageType(
                "SensorReading".to_owned(),
              ))
              .into(),
            )
          }
        }
        .boxed();
      }
    }
  }
      create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(
          ButtplugDeviceMessageNameV4::SensorCmd.to_string(),
        )
        .into(),
      )
  }

  pub fn battery_level(&self) -> ButtplugClientResultFuture<u32> {
    if self.feature().sensor().as_ref().ok_or(false).unwrap().contains_key(&SensorType::Battery) {
      let send_fut = self.read_sensor(SensorType::Battery);
      Box::pin(async move {
        let data = send_fut.await?;
        let battery_level = data[0];
        Ok(battery_level as u32)
      })
    } else {
      create_boxed_future_client_error(
        ButtplugDeviceError::DeviceFeatureMismatch("Device feature is not battery".to_owned())
          .into(),
      )
    }
  }

  pub fn rssi_level(&self) -> ButtplugClientResultFuture<u32> {
    if self.feature().sensor().as_ref().ok_or(false).unwrap().contains_key(&SensorType::RSSI) {
      let send_fut = self.read_sensor(SensorType::RSSI);
      Box::pin(async move {
        let data = send_fut.await?;
        let battery_level = data[0];
        Ok(battery_level as u32)
      })
    } else {
      create_boxed_future_client_error(
        ButtplugDeviceError::DeviceFeatureMismatch("Device feature is not RSSI".to_owned()).into(),
      )
    }
  }
}

