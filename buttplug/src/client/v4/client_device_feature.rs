use std::sync::Arc;

use futures::{future, FutureExt};
use getset::{CopyGetters, Getters};

use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
    message::{
      ButtplugSensorFeatureMessageType,
      ButtplugServerMessageV4,
      DeviceFeature,
      FeatureType,
      ValueCmdV4,
      ValueSubcommandV4,
      SensorReadCmdV4,
      SensorSubscribeCmdV4,
      SensorUnsubscribeCmdV4,
    },
  },
  server::message::ButtplugDeviceMessageType,
};

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

  pub(super) fn value_subcommand(&self, value: i32) -> ValueSubcommandV4 {
    ValueSubcommandV4::new(self.feature_index, value)
  }

  fn check_and_set_value(
    &self,
    feature: FeatureType,
    value: i32,
  ) -> ButtplugClientResultFuture {
    if *self.feature.feature_type() != feature {
      future::ready(Err(ButtplugClientError::from(ButtplugError::from(
        ButtplugDeviceError::DeviceActuatorTypeMismatch(
          self.feature_index,
          feature.try_into().unwrap(),
          *self.feature.feature_type(),
        ),
      ))))
      .boxed()
    } else {
      self.event_loop_sender.send_message_expect_ok(
        ValueCmdV4::new(self.device_index, vec![self.value_subcommand(value)]).into(),
      )
    }
  }

  pub fn vibrate(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_value(FeatureType::Vibrate, level as i32)
  }

  pub fn oscillate(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_value(FeatureType::Oscillate, level as i32)
  }

  pub fn rotate(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_value(FeatureType::Rotate, level as i32)
  }

  pub fn inflate(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_value(FeatureType::Inflate, level as i32)
  }

  pub fn constrict(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_value(FeatureType::Constrict, level as i32)
  }

  pub fn position(&self, level: u32) -> ButtplugClientResultFuture {
    self.check_and_set_value(FeatureType::Position, level as i32)
  }

  pub fn rotate_with_direction(&self, level: i32) -> ButtplugClientResultFuture {
    self.check_and_set_value(FeatureType::RotateWithDirection, level)
  }

  pub fn subscribe_sensor(&self, sensor_index: u32) -> ButtplugClientResultFuture {
    if let Some(sensor) = self.feature.sensor() {
      if sensor
        .messages()
        .contains(&ButtplugSensorFeatureMessageType::SensorReadCmd)
      {
        let msg = SensorSubscribeCmdV4::new(
          self.device_index,
          sensor_index,
          (*self.feature.feature_type()).try_into().unwrap(),
        )
        .into();
        self.event_loop_sender.send_message_expect_ok(msg)
      } else {
        create_boxed_future_client_error(
          ButtplugDeviceError::MessageNotSupported(
            ButtplugDeviceMessageType::SensorSubscribeCmd.to_string(),
          )
          .into(),
        )
      }
    } else {
      create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(
          ButtplugDeviceMessageType::SensorSubscribeCmd.to_string(),
        )
        .into(),
      )
    }
  }

  pub fn unsubscribe_sensor(&self, sensor_index: u32) -> ButtplugClientResultFuture {
    if let Some(sensor) = self.feature.sensor() {
      if sensor
        .messages()
        .contains(&ButtplugSensorFeatureMessageType::SensorReadCmd)
      {
        let msg = SensorUnsubscribeCmdV4::new(
          self.device_index,
          sensor_index,
          (*self.feature.feature_type()).try_into().unwrap(),
        )
        .into();
        self.event_loop_sender.send_message_expect_ok(msg)
      } else {
        create_boxed_future_client_error(
          ButtplugDeviceError::MessageNotSupported(
            ButtplugDeviceMessageType::SensorSubscribeCmd.to_string(),
          )
          .into(),
        )
      }
    } else {
      create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(
          ButtplugDeviceMessageType::SensorSubscribeCmd.to_string(),
        )
        .into(),
      )
    }
  }

  fn read_single_sensor(&self) -> ButtplugClientResultFuture<Vec<i32>> {
    if let Some(sensor) = self.feature.sensor() {
      if sensor
        .messages()
        .contains(&ButtplugSensorFeatureMessageType::SensorReadCmd)
      {
        let msg = SensorReadCmdV4::new(
          self.device_index,
          self.feature_index,
          (*self.feature().feature_type()).try_into().unwrap(),
        )
        .into();
        let reply = self.event_loop_sender.send_message(msg);
        async move {
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
        .boxed()
      } else {
        create_boxed_future_client_error(
          ButtplugDeviceError::MessageNotSupported(
            ButtplugDeviceMessageType::SensorSubscribeCmd.to_string(),
          )
          .into(),
        )
      }
    } else {
      create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(
          ButtplugDeviceMessageType::SensorSubscribeCmd.to_string(),
        )
        .into(),
      )
    }
  }

  fn has_sensor_read(&self) -> bool {
    if let Some(sensor) = self.feature.sensor() {
      sensor
        .messages()
        .contains(&ButtplugSensorFeatureMessageType::SensorReadCmd)
    } else {
      false
    }
  }

  pub fn battery_level(&self) -> ButtplugClientResultFuture<u32> {
    if *self.feature.feature_type() == FeatureType::Battery && self.has_sensor_read() {
      let send_fut = self.read_single_sensor();
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
    if *self.feature.feature_type() == FeatureType::RSSI && self.has_sensor_read() {
      let send_fut = self.read_single_sensor();
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
