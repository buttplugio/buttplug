// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::{ButtplugDeviceMessageType, Endpoint};
use getset::{Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize, Serializer, ser::SerializeSeq};
use std::{collections::HashSet, ops::RangeInclusive};

use super::{ActuatorType, SensorType};

#[derive(Debug, Default, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureType {
  #[default]
  Unknown,
  Vibrate,
  // Single Direction Rotation Speed
  Rotate,
  Oscillate,
  Constrict,
  Inflate,
  // For instances where we specify a position to move to ASAP. Usually servos, probably for the
  // OSR-2/SR-6.
  Position,
  // Sensor Types
  Battery,
  RSSI,
  Button,
  Pressure,
  // Currently unused but possible sensor features:
  // Temperature,
  // Accelerometer,
  // Gyro,
  //
  // Raw Feature, for when raw messages are on
  Raw,
}

impl From<ActuatorType> for FeatureType {
  fn from(value: ActuatorType) -> Self {
    match value {
      ActuatorType::Unknown => FeatureType::Unknown,
      ActuatorType::Vibrate => FeatureType::Vibrate,
      ActuatorType::Rotate => FeatureType::Rotate,
      ActuatorType::Oscillate => FeatureType::Oscillate,
      ActuatorType::Constrict => FeatureType::Constrict,
      ActuatorType::Inflate => FeatureType::Inflate,
      ActuatorType::Position => FeatureType::Position,
    }
  }
}

impl From<SensorType> for FeatureType {
  fn from(value: SensorType) -> Self {
    match value {
      SensorType::Unknown => FeatureType::Unknown,
      SensorType::Battery => FeatureType::Battery,
      SensorType::RSSI => FeatureType::RSSI,
      SensorType::Button => FeatureType::Button,
      SensorType::Pressure => FeatureType::Pressure,
    }
  }
}

// This will look almost exactly like ServerDeviceFeature. However, it will only contain
// information we want the client to know, i.e. step counts versus specific step ranges. This is
// what will be sent to the client as part of DeviceAdded/DeviceList messages. It should not be used
// for outside configuration/serialization, rather it should be a subset of that information.
//
// For many messages, client and server configurations may be exactly the same. If they are not,
// then we denote this by prefixing the type with Client/Server. Server attributes will usually be
// hosted in the server/device/configuration module.
#[derive(
  Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize,
)]
pub struct DeviceFeature {
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(default)]
  description: String,
  #[getset(get = "pub")]
  #[serde(rename = "feature-type")]
  feature_type: FeatureType,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "actuator")]
  actuator: Option<DeviceFeatureActuator>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "sensor")]
  sensor: Option<DeviceFeatureSensor>,
  #[getset(get = "pub")]
  #[serde(skip)]
  raw: Option<DeviceFeatureRaw>,
}

impl DeviceFeature {
  pub fn new(
    description: &str,
    feature_type: FeatureType,
    actuator: &Option<DeviceFeatureActuator>,
    sensor: &Option<DeviceFeatureSensor>,
  ) -> Self {
    Self {
      description: description.to_owned(),
      feature_type,
      actuator: actuator.clone(),
      sensor: sensor.clone(),
      raw: None,
    }
  }

  pub fn new_raw_feature(endpoints: &[Endpoint]) -> Self {
    Self {
      description: "Raw Endpoints".to_owned(),
      feature_type: FeatureType::Raw,
      actuator: None,
      sensor: None,
      raw: Some(DeviceFeatureRaw::new(endpoints)),
    }
  }
}

fn range_serialize<S>(
  range: &RangeInclusive<u32>,
  serializer: S,
) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  let mut seq = serializer.serialize_seq(Some(2))?;
  seq.serialize_element(&range.start())?;
  seq.serialize_element(&range.end())?;
  seq.end()
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

#[derive(Clone, Debug, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize)]
pub struct DeviceFeatureActuator {
  #[getset(get = "pub")]
  #[serde(rename = "step-range")]
  #[serde(serialize_with="range_serialize")]
  step_range: RangeInclusive<u32>,
  // This will only exist in user configs, therefore it's an Option<T>
  #[getset(get = "pub")]
  #[serde(rename = "step-limit")]
  #[serde(skip_serializing_if = "Option::is_none")]
  //#[serde(serialize_with="range_serialize")]
  step_limit: Option<RangeInclusive<u32>>,
  #[getset(get = "pub")]
  #[serde(rename = "messages")]
  messages: HashSet<ButtplugDeviceMessageType>,
}

impl DeviceFeatureActuator {
  pub fn new(
    step_range: &RangeInclusive<u32>,
    messages: &HashSet<ButtplugDeviceMessageType>,
  ) -> Self {
    Self {
      step_range: step_range.clone(),
      step_limit: None,
      messages: messages.clone(),
    }
  }
}

#[derive(
  Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize,
)]
pub struct DeviceFeatureSensor {
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "value-range")]
  #[serde(serialize_with="range_sequence_serialize")]
  value_range: Vec<RangeInclusive<i32>>,
  #[getset(get = "pub")]
  #[serde(rename = "messages")]
  messages: HashSet<ButtplugDeviceMessageType>,
}

impl DeviceFeatureSensor {
  pub fn new(
    value_range: &Vec<RangeInclusive<i32>>,
    messages: &HashSet<ButtplugDeviceMessageType>,
  ) -> Self {
    Self {
      value_range: value_range.clone(),
      messages: messages.clone(),
    }
  }
}

#[derive(
  Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize,
)]
pub struct DeviceFeatureRaw {
  #[getset(get = "pub")]
  #[serde(rename = "Endpoints")]
  endpoints: Vec<Endpoint>,
  #[getset(get = "pub")]
  #[serde(rename = "Messages")]
  messages: HashSet<ButtplugDeviceMessageType>,
}

impl DeviceFeatureRaw {
  pub fn new(endpoints: &[Endpoint]) -> Self {
    Self {
      endpoints: endpoints.into(),
      messages: HashSet::from_iter(
        [
          ButtplugDeviceMessageType::RawReadCmd,
          ButtplugDeviceMessageType::RawWriteCmd,
          ButtplugDeviceMessageType::RawSubscribeCmd,
          ButtplugDeviceMessageType::RawUnsubscribeCmd,
        ]
        .iter()
        .cloned(),
      ),
    }
  }
}
