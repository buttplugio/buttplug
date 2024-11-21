// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::{
  errors::ButtplugDeviceError,
  message::{
    ActuatorType,
    ButtplugActuatorFeatureMessageType,
    ButtplugDeviceMessageType,
    ButtplugSensorFeatureMessageType,
    DeviceFeature,
    NullDeviceMessageAttributesV1,
    RawDeviceMessageAttributesV2,
    SensorType,
  },
};
use getset::{Getters, MutGetters, Setters};
use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
use std::ops::RangeInclusive;

// This will look almost exactly like ServerDeviceMessageAttributes. However, it will only contain
// information we want the client to know, i.e. step counts versus specific step ranges. This is
// what will be sent to the client as part of DeviceAdded/DeviceList messages. It should not be used
// for outside configuration/serialization, rather it should be a subset of that information.
//
// For many messages, client and server configurations may be exactly the same. If they are not,
// then we denote this by prefixing the type with Client/Server. Server attributes will usually be
// hosted in the server/device/configuration module.
#[derive(Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct ClientDeviceMessageAttributesV3 {
  // Generic commands
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "ScalarCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  scalar_cmd: Option<Vec<ClientGenericDeviceMessageAttributesV3>>,
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "RotateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  rotate_cmd: Option<Vec<ClientGenericDeviceMessageAttributesV3>>,
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "LinearCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  linear_cmd: Option<Vec<ClientGenericDeviceMessageAttributesV3>>,

  // Sensor Messages
  #[getset(get = "pub")]
  #[serde(rename = "SensorReadCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  sensor_read_cmd: Option<Vec<SensorDeviceMessageAttributesV3>>,
  #[getset(get = "pub")]
  #[serde(rename = "SensorSubscribeCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  sensor_subscribe_cmd: Option<Vec<SensorDeviceMessageAttributesV3>>,

  // StopDeviceCmd always exists
  #[getset(get = "pub")]
  #[serde(rename = "StopDeviceCmd")]
  #[serde(skip_deserializing)]
  stop_device_cmd: NullDeviceMessageAttributesV1,

  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(rename = "RawReadCmd")]
  #[serde(skip_deserializing)]
  #[serde(skip_serializing_if = "Option::is_none")]
  raw_read_cmd: Option<RawDeviceMessageAttributesV2>,
  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(rename = "RawWriteCmd")]
  #[serde(skip_deserializing)]
  #[serde(skip_serializing_if = "Option::is_none")]
  raw_write_cmd: Option<RawDeviceMessageAttributesV2>,
  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(rename = "RawSubscribeCmd")]
  #[serde(skip_deserializing)]
  #[serde(skip_serializing_if = "Option::is_none")]
  raw_subscribe_cmd: Option<RawDeviceMessageAttributesV2>,

  // Needed to load from config for fallback, but unused here.
  #[getset(get = "pub")]
  #[serde(rename = "FleshlightLaunchFW12Cmd")]
  #[serde(skip_serializing)]
  fleshlight_launch_fw12_cmd: Option<NullDeviceMessageAttributesV1>,
  #[getset(get = "pub")]
  #[serde(rename = "VorzeA10CycloneCmd")]
  #[serde(skip_serializing)]
  vorze_a10_cyclone_cmd: Option<NullDeviceMessageAttributesV1>,
}

impl From<Vec<DeviceFeature>> for ClientDeviceMessageAttributesV3 {
  fn from(features: Vec<DeviceFeature>) -> Self {
    let actuator_filter = |message_type| {
      let attrs: Vec<ClientGenericDeviceMessageAttributesV3> = features
        .iter()
        .filter(|x| {
          if let Some(actuator) = x.actuator() {
            actuator.messages().contains(message_type)
          } else {
            false
          }
        })
        .map(|x| x.clone().try_into().unwrap())
        .collect();
      if !attrs.is_empty() {
        Some(attrs)
      } else {
        None
      }
    };

    let sensor_filter = |message_type| {
      let attrs: Vec<SensorDeviceMessageAttributesV3> = features
        .iter()
        .filter(|x| {
          if let Some(sensor) = x.sensor() {
            sensor.messages().contains(message_type)
          } else {
            false
          }
        })
        .map(|x| x.clone().try_into().unwrap())
        .collect();
      if !attrs.is_empty() {
        Some(attrs)
      } else {
        None
      }
    };

    // Raw messages
    let raw_attrs = features.iter().find(|f| f.raw().is_some()).map(|raw_feature| RawDeviceMessageAttributesV2::new(
        raw_feature.raw().as_ref().unwrap().endpoints(),
      ));

    Self {
      scalar_cmd: actuator_filter(&ButtplugActuatorFeatureMessageType::ScalarCmd),
      rotate_cmd: actuator_filter(&ButtplugActuatorFeatureMessageType::RotateCmd),
      linear_cmd: actuator_filter(&ButtplugActuatorFeatureMessageType::LinearCmd),
      sensor_read_cmd: sensor_filter(&ButtplugSensorFeatureMessageType::SensorReadCmd),
      sensor_subscribe_cmd: sensor_filter(&ButtplugSensorFeatureMessageType::SensorSubscribeCmd),
      raw_read_cmd: raw_attrs.clone(),
      raw_write_cmd: raw_attrs.clone(),
      raw_subscribe_cmd: raw_attrs.clone(),
      ..Default::default()
    }
  }
}

impl ClientDeviceMessageAttributesV3 {
  pub fn raw_unsubscribe_cmd(&self) -> &Option<RawDeviceMessageAttributesV2> {
    self.raw_subscribe_cmd()
  }

  pub fn message_allowed(&self, message_type: &ButtplugDeviceMessageType) -> bool {
    match message_type {
      ButtplugDeviceMessageType::ScalarCmd => self.scalar_cmd.is_some(),
      // VibrateCmd and SingleMotorVibrateCmd will derive from Scalars, so errors will be thrown in
      // the scalar parser if the actuator isn't correct.
      ButtplugDeviceMessageType::VibrateCmd => self.scalar_cmd.is_some(),
      ButtplugDeviceMessageType::SingleMotorVibrateCmd => self.scalar_cmd.is_some(),
      ButtplugDeviceMessageType::SensorReadCmd => self.sensor_read_cmd.is_some(),
      ButtplugDeviceMessageType::SensorSubscribeCmd => self.sensor_subscribe_cmd.is_some(),
      ButtplugDeviceMessageType::SensorUnsubscribeCmd => self.sensor_subscribe_cmd.is_some(),
      ButtplugDeviceMessageType::LinearCmd => self.linear_cmd.is_some(),
      ButtplugDeviceMessageType::RotateCmd => self.rotate_cmd.is_some(),
      ButtplugDeviceMessageType::BatteryLevelCmd => {
        if let Some(sensor_info) = &self.sensor_read_cmd {
          sensor_info
            .iter()
            .any(|x| *x.sensor_type() == SensorType::Battery)
        } else {
          false
        }
      }
      ButtplugDeviceMessageType::FleshlightLaunchFW12Cmd => {
        self.fleshlight_launch_fw12_cmd.is_some()
      }
      ButtplugDeviceMessageType::RSSILevelCmd => {
        if let Some(sensor_info) = &self.sensor_read_cmd {
          sensor_info
            .iter()
            .any(|x| *x.sensor_type() == SensorType::RSSI)
        } else {
          false
        }
      }
      ButtplugDeviceMessageType::RawReadCmd => self.raw_read_cmd.is_some(),
      ButtplugDeviceMessageType::RawSubscribeCmd => self.raw_subscribe_cmd.is_some(),
      ButtplugDeviceMessageType::RawUnsubscribeCmd => self.raw_subscribe_cmd.is_some(),
      ButtplugDeviceMessageType::RawWriteCmd => self.raw_write_cmd.is_some(),
      ButtplugDeviceMessageType::VorzeA10CycloneCmd => self.vorze_a10_cyclone_cmd.is_some(),
      ButtplugDeviceMessageType::StopDeviceCmd => true,
      ButtplugDeviceMessageType::KiirooCmd => false,
      ButtplugDeviceMessageType::LovenseCmd => false,
    }
  }

  pub fn finalize(&mut self) {
    if let Some(scalar_attrs) = &mut self.scalar_cmd {
      for (i, attr) in scalar_attrs.iter_mut().enumerate() {
        attr.index = i as u32;
      }
    }
    if let Some(sensor_read_attrs) = &mut self.sensor_read_cmd {
      for (i, attr) in sensor_read_attrs.iter_mut().enumerate() {
        attr.index = i as u32;
      }
    }
    if let Some(sensor_subscribe_attrs) = &mut self.sensor_subscribe_cmd {
      for (i, attr) in sensor_subscribe_attrs.iter_mut().enumerate() {
        attr.index = i as u32;
      }
    }
  }
}

fn unspecified_feature() -> String {
  "N/A".to_string()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct ClientGenericDeviceMessageAttributesV3 {
  #[getset(get = "pub")]
  #[serde(rename = "FeatureDescriptor")]
  #[serde(default = "unspecified_feature")]
  feature_descriptor: String,
  #[getset(get = "pub")]
  #[serde(rename = "ActuatorType")]
  actuator_type: ActuatorType,
  #[serde(rename = "StepCount")]
  #[getset(get = "pub")]
  step_count: u32,
  // TODO This needs to actually be part of the device info relayed to the client in spec v4.
  #[getset(get = "pub")]
  #[serde(skip, default)]
  index: u32,
}

impl TryFrom<DeviceFeature> for ClientGenericDeviceMessageAttributesV3 {
  type Error = String;
  fn try_from(value: DeviceFeature) -> Result<Self, Self::Error> {
    if let Some(actuator) = value.actuator() {
      let actuator_type = (*value.feature_type()).try_into()?;
      let step_limit = actuator.step_limit();
      let step_count = step_limit.end() - step_limit.start();
      let attrs = Self {
        feature_descriptor: value.description().to_owned(),
        actuator_type,
        step_count,
        index: 0,
      };
      Ok(attrs)
    } else {
      Err("Cannot produce a GenericDeviceMessageAttribute from a feature with no actuator member".to_string())
    }
  }
}

impl ClientGenericDeviceMessageAttributesV3 {
  pub fn new(feature_descriptor: &str, step_count: u32, actuator_type: ActuatorType) -> Self {
    Self {
      feature_descriptor: feature_descriptor.to_owned(),
      actuator_type,
      step_count,
      index: 0,
    }
  }

  // This is created out of already verified server device message attributes, so we'll assume it's
  // fine.
  pub fn is_valid(&self, _: &ButtplugDeviceMessageType) -> Result<(), ButtplugDeviceError> {
    Ok(())
  }
}

fn range_sequence_serialize<S>(
  range_vec: &Vec<RangeInclusive<i32>>,
  serializer: S,
) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  let mut seq = serializer.serialize_seq(Some(range_vec.len()))?;
  for range in range_vec {
    seq.serialize_element(&vec![*range.start(), *range.end()])?;
  }
  seq.end()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct SensorDeviceMessageAttributesV3 {
  #[getset(get = "pub")]
  #[serde(rename = "FeatureDescriptor")]
  feature_descriptor: String,
  #[getset(get = "pub")]
  #[serde(rename = "SensorType")]
  sensor_type: SensorType,
  #[getset(get = "pub")]
  #[serde(rename = "SensorRange", serialize_with = "range_sequence_serialize")]
  sensor_range: Vec<RangeInclusive<i32>>,
  // TODO This needs to actually be part of the device info relayed to the client in spec v4.
  #[getset(get = "pub")]
  #[serde(skip, default)]
  index: u32,
}

impl TryFrom<DeviceFeature> for SensorDeviceMessageAttributesV3 {
  type Error = String;
  fn try_from(value: DeviceFeature) -> Result<Self, Self::Error> {
    if let Some(sensor) = value.sensor() {
      Ok(Self {
        feature_descriptor: value.description().to_owned(),
        sensor_type: (*value.feature_type()).try_into()?,
        sensor_range: sensor.value_range().clone(),
        index: 0,
      })
    } else {
      Err("Device Feature does not expose a sensor.".to_owned())
    }
  }
}

/*
impl SensorDeviceMessageAttributes {
  pub fn new(feature_descriptor: &str, sensor_type: SensorType) -> Self {
    Self { feature_descriptor: feature_descriptor.to_owned(), sensor_type }
  }
}
 */
