// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::configuration::ProtocolDeviceAttributes;
use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{
      self, ButtplugActuatorFeatureMessageType, ButtplugDeviceMessage, ButtplugMessage,
      ButtplugSensorFeatureMessageType, ButtplugServerDeviceMessage, ButtplugServerMessage,
      DeviceFeature, DeviceFeatureActuator, DeviceFeatureSensor, Endpoint, FeatureType,
      SensorReading, SensorType,
    },
  },
  server::device::{
    configuration::{UserDeviceCustomization, UserDeviceDefinition, UserDeviceIdentifier},
    hardware::{
      Hardware, HardwareCommand, HardwareEvent, HardwareReadCmd, HardwareSubscribeCmd,
      HardwareUnsubscribeCmd, HardwareWriteCmd,
    },
    protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
  },
  util::{async_manager, stream::convert_broadcast_receiver_to_stream},
};
use async_trait::async_trait;
use dashmap::DashSet;
use futures::{
  future::{self, BoxFuture},
  FutureExt, StreamExt,
};
use minicbor::Decode;
use std::{collections::HashSet, ops::RangeInclusive, pin::Pin, sync::Arc};
use tokio::sync::broadcast;

const READ_CMD_TIMEOUT_MS: u32 = 500;
const MAX_FIRMWARE_DESCRIPTOR_LEN: u32 = 128;

#[derive(Debug, Clone, Decode)]
struct GBleActuatorFeature {
  #[n(0)]
  description: String,
  #[n(1)]
  feature_type: u8,
  #[n(2)]
  step_range_low: u32,
  #[n(3)]
  step_range_high: u32,
  #[n(4)]
  message_type: u8,
}

impl GBleActuatorFeature {
  fn to_device_feature(&self) -> Result<DeviceFeature, ButtplugDeviceError> {
    let feature_type = match self.feature_type {
      1 => FeatureType::Vibrate,
      2 => FeatureType::Rotate,
      3 => FeatureType::Oscillate,
      4 => FeatureType::Constrict,
      5 => FeatureType::Inflate,
      6 => FeatureType::Position,
      x => {
        error!("GenericBtle invalid feature type for actuator {:?}", x);
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "GenericBtle".to_owned(),
          "Invalid actuator feature type".to_owned(),
        ));
      }
    };

    let message_type = match self.message_type {
      1 => ButtplugActuatorFeatureMessageType::ScalarCmd,
      2 => ButtplugActuatorFeatureMessageType::RotateCmd,
      3 => ButtplugActuatorFeatureMessageType::LinearCmd,
      x => {
        error!("GenericBtle invalid message type for actuator {:?}", x);
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "GenericBtle".to_owned(),
          "Invalid actuator message type".to_owned(),
        ));
      }
    };

    let step_range = RangeInclusive::<u32>::new(self.step_range_low, self.step_range_high);

    Ok(DeviceFeature::new(
      self.description.as_str(),
      feature_type,
      &Some(DeviceFeatureActuator::new(
        &step_range,
        &step_range,
        &HashSet::from_iter([message_type]),
      )),
      &None,
    ))
  }
}

#[derive(Debug, Clone, Decode)]
struct GBleSensorFeature {
  #[n(0)]
  description: String,
  #[n(1)]
  feature_type: u8,
  #[n(2)]
  value_range_low: i32,
  #[n(3)]
  value_range_high: i32,
  #[n(4)]
  message_type: u8,
}

impl GBleSensorFeature {
  fn to_device_feature(&self) -> Result<DeviceFeature, ButtplugDeviceError> {
    let feature_type = match self.feature_type {
      1 => FeatureType::Battery,
      2 => FeatureType::RSSI,
      3 => FeatureType::Button,
      4 => FeatureType::Pressure,
      x => {
        error!("GenericBtle invalid feature type for sensor {:?}", x);
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "GenericBtle".to_owned(),
          "Invalid sensor feature type".to_owned(),
        ));
      }
    };

    let message_type = match self.message_type {
      1 => ButtplugSensorFeatureMessageType::SensorReadCmd,
      2 => ButtplugSensorFeatureMessageType::SensorSubscribeCmd,
      x => {
        error!("GenericBtle invalid message type for sensor {:?}", x);
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "GenericBtle".to_owned(),
          "Invalid sensor message type".to_owned(),
        ));
      }
    };

    let value_range = RangeInclusive::<i32>::new(self.value_range_low, self.value_range_high);

    Ok(DeviceFeature::new(
      self.description.as_str(),
      feature_type,
      &None,
      &Some(DeviceFeatureSensor::new(
        &vec![value_range],
        &HashSet::from_iter([message_type]),
      )),
    ))
  }
}

#[derive(Debug, Clone, Decode)]
struct GBleDescriptor {
  #[n(0)]
  _version: u8,
  #[n(1)]
  dev_name: String,
  #[n(2)]
  actuator_features: Vec<GBleActuatorFeature>,
  #[n(3)]
  sensor_features: Vec<GBleSensorFeature>,
}

impl GBleDescriptor {
  fn to_device_def(&self) -> Result<UserDeviceDefinition, ButtplugDeviceError> {
    let actuators = self
      .actuator_features
      .to_owned()
      .into_iter()
      .map(|f| f.to_device_feature())
      .collect::<Result<Vec<DeviceFeature>, ButtplugDeviceError>>()?;

    let sensors = self
      .sensor_features
      .to_owned()
      .into_iter()
      .map(|f| f.to_device_feature())
      .collect::<Result<Vec<DeviceFeature>, ButtplugDeviceError>>()?;

    let features = [actuators, sensors].concat();

    Ok(UserDeviceDefinition::new(
      &self.dev_name.as_str(),
      features.as_slice(),
      &UserDeviceCustomization::new(&None, true, false, 0),
    ))
  }
}

#[derive(Debug, Clone, Decode)]
struct GBleSensorReading {
  #[n(0)]
  sensor_id: u32,
  #[n(1)]
  value: i32,
}

pub mod setup {
  use crate::server::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct GenericBtleIdentifierFactory {}

  impl ProtocolIdentifierFactory for GenericBtleIdentifierFactory {
    fn identifier(&self) -> &str {
      "generic-btle"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::GenericBtleIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct GenericBtleIdentifier {}

#[async_trait]
impl ProtocolIdentifier for GenericBtleIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let read_descriptor_cmd = HardwareReadCmd::new(
      Endpoint::Firmware,
      MAX_FIRMWARE_DESCRIPTOR_LEN,
      READ_CMD_TIMEOUT_MS,
    );

    let Ok(descriptor_reading) = hardware.read_value(&read_descriptor_cmd).await else {
      return Err(ButtplugDeviceError::ProtocolSpecificError(
        "GenericBtle".to_owned(),
        "Failed to read firmware descriptor".to_owned(),
      ));
    };

    let descriptor = match minicbor::decode::<GBleDescriptor>(descriptor_reading.data().as_slice())
    {
      Ok(v) => v,
      Err(e) => {
        error!("GenericBtle failed to parse descriptor {:?}", e);
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "GenericBtle".to_owned(),
          "Failed to parse descriptor".to_owned(),
        ));
      }
    };

    return Ok((
      UserDeviceIdentifier::new(hardware.address(), &descriptor.dev_name.as_str(), &None),
      Box::new(GenericBtleInitializer::new()),
    ));
  }

  async fn define(
    &mut self,
    hardware: Arc<Hardware>,
    _identifier: &UserDeviceIdentifier,
    _raw_endpoints: &[Endpoint],
  ) -> Result<Option<UserDeviceDefinition>, ButtplugDeviceError> {
    // Read the device descriptor from the Firmware endpoint
    let read_descriptor_cmd = HardwareReadCmd::new(
      Endpoint::Firmware,
      MAX_FIRMWARE_DESCRIPTOR_LEN,
      READ_CMD_TIMEOUT_MS,
    );

    let Ok(descriptor_reading) = hardware.read_value(&read_descriptor_cmd).await else {
      return Err(ButtplugDeviceError::ProtocolSpecificError(
        "GenericBtle".to_owned(),
        "Failed to read firmware descriptor".to_owned(),
      ));
    };

    let descriptor = match minicbor::decode::<GBleDescriptor>(descriptor_reading.data().as_slice())
    {
      Ok(v) => v,
      Err(e) => {
        error!("GenericBtle failed to parse descriptor {:?}", e);
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "GenericBtle".to_owned(),
          "Failed to parse descriptor".to_owned(),
        ));
      }
    };

    let definition = descriptor.to_device_def()?;

    Ok(Some(definition))
  }
}
pub struct GenericBtleInitializer {}

impl GenericBtleInitializer {
  pub fn new() -> Self {
    Self {}
  }
}

#[async_trait]
impl ProtocolInitializer for GenericBtleInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    _attributes: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(GenericBtle::default()))
  }
}

pub struct GenericBtle {
  // Set of sensors we've subscribed to for updates.
  subscribed_sensors: Arc<DashSet<u32>>,
  event_stream: broadcast::Sender<ButtplugServerDeviceMessage>,
}

impl Default for GenericBtle {
  fn default() -> Self {
    let (sender, _) = broadcast::channel(256);
    Self {
      subscribed_sensors: Arc::new(DashSet::new()),
      event_stream: sender,
    }
  }
}

impl ProtocolHandler for GenericBtle {
  fn event_stream(
    &self,
  ) -> Pin<Box<dyn futures::Stream<Item = ButtplugServerDeviceMessage> + Send>> {
    convert_broadcast_receiver_to_stream(self.event_stream.subscribe()).boxed()
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let cmd_data = minicbor::to_vec((index, scalar)).unwrap();

    Ok(vec![
      HardwareWriteCmd::new(Endpoint::Tx, cmd_data, false).into()
    ])
  }

  fn handle_sensor_subscribe_cmd(
    &self,
    device: Arc<Hardware>,
    message: message::SensorSubscribeCmd,
  ) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
    debug!("GenericBtle subscribe func",);

    if self.subscribed_sensors.contains(message.sensor_index()) {
      return future::ready(Ok(message::Ok::new(message.id()).into())).boxed();
    }
    let sensors = self.subscribed_sensors.clone();
    async move {
      // If we have no sensors we're currently subscribed to, we'll need to bring up our BLE
      // characteristic subscription.
      if sensors.is_empty() {
        device
          .subscribe(&HardwareSubscribeCmd::new(Endpoint::Rx))
          .await?;
        let sender = self.event_stream.clone();
        let mut hardware_stream = device.event_stream();
        let stream_sensors = sensors.clone();
        let device_index = message.device_index();

        // If we subscribe successfully, we need to set up our event handler.
        async_manager::spawn(async move {
          while let Ok(info) = hardware_stream.recv().await {
            // If we have no receivers, quit.
            if sender.receiver_count() == 0 || stream_sensors.is_empty() {
              return;
            }
            if let HardwareEvent::Notification(_, endpoint, data) = info {
              if endpoint == Endpoint::Rx {
                let gble_reading = match minicbor::decode::<GBleSensorReading>(data.as_slice()) {
                  Ok(v) => v,
                  Err(e) => {
                    error!("GenericBtle failed to parse sensor reading {:?}", e);
                    return;
                  }
                };

                let reading = SensorReading::new(
                  device_index,
                  gble_reading.sensor_id,
                  SensorType::Pressure,
                  vec![gble_reading.value],
                );

                if sender.send(reading.into()).is_err() {
                  error!(
                    "Hardware device listener for GenericBtle shut down, returning from task."
                  );
                  return;
                }
              }
            }
          }
        });
      }
      sensors.insert(*message.sensor_index());
      Ok(message::Ok::new(message.id()).into())
    }
    .boxed()
  }

  fn handle_sensor_unsubscribe_cmd(
    &self,
    device: Arc<Hardware>,
    message: message::SensorUnsubscribeCmd,
  ) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
    if !self.subscribed_sensors.contains(message.sensor_index()) {
      return future::ready(Ok(message::Ok::new(message.id()).into())).boxed();
    }
    let sensors = self.subscribed_sensors.clone();
    async move {
      // If we have no sensors we're currently subscribed to, we'll need to bring up our BLE
      // characteristic subscription.
      sensors.remove(message.sensor_index());
      if sensors.is_empty() {
        device
          .unsubscribe(&HardwareUnsubscribeCmd::new(Endpoint::Rx))
          .await?;
      }
      Ok(message::Ok::new(message.id()).into())
    }
    .boxed()
  }
}
