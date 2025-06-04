// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::{Endpoint, SensorCommandType};
use getset::{Getters, MutGetters, Setters};
use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
use std::{collections::{HashMap, HashSet}, ops::RangeInclusive};

#[derive(Debug, Default, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureType {
  #[default]
  // Used for when types are added that we do not know how to handle
  Unknown,
  // Level/ValueCmd types
  Vibrate,
  // Single Direction Rotation Speed
  Rotate,
  Oscillate,
  Constrict,
  Inflate,
  Heater,
  Led,
  // For instances where we specify a position to move to ASAP. Usually servos, probably for the
  // OSR-2/SR-6.
  Position,
  // ValueWithParameterCmd types
  // Two Direction Rotation Speed
  RotateWithDirection,
  PositionWithDuration,
  // Might be useful but dunno if we need it yet, or how to convey "speed" units
  // PositionWithSpeed
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


#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ActuatorType {
  Unknown,
  Vibrate,
  // Single Direction Rotation Speed
  Rotate,
  // Two Direction Rotation Speed
  RotateWithDirection,
  Oscillate,
  Constrict,
  Inflate,
  Heater,
  Led,
  // For instances where we specify a position to move to ASAP. Usually servos, probably for the
  // OSR-2/SR-6.
  Position,
  PositionWithDuration,
}

impl TryFrom<FeatureType> for ActuatorType {
  type Error = String;
  fn try_from(value: FeatureType) -> Result<Self, Self::Error> {
    match value {
      FeatureType::Unknown => Ok(ActuatorType::Unknown),
      FeatureType::Vibrate => Ok(ActuatorType::Vibrate),
      FeatureType::Rotate => Ok(ActuatorType::Rotate),
      FeatureType::Heater => Ok(ActuatorType::Heater),
      FeatureType::Led => Ok(ActuatorType::Led),
      FeatureType::RotateWithDirection => Ok(ActuatorType::RotateWithDirection),
      FeatureType::PositionWithDuration => Ok(ActuatorType::PositionWithDuration),
      FeatureType::Oscillate => Ok(ActuatorType::Oscillate),
      FeatureType::Constrict => Ok(ActuatorType::Constrict),
      FeatureType::Inflate => Ok(ActuatorType::Inflate),
      FeatureType::Position => Ok(ActuatorType::Position),
      _ => Err(format!(
        "Feature type {value} not valid for ActuatorType conversion"
      )),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, Hash)]
pub enum SensorType {
  Unknown,
  Battery,
  RSSI,
  Button,
  Pressure,
  // Temperature,
  // Accelerometer,
  // Gyro,
}

impl TryFrom<FeatureType> for SensorType {
  type Error = String;
  fn try_from(value: FeatureType) -> Result<Self, Self::Error> {
    match value {
      FeatureType::Unknown => Ok(SensorType::Unknown),
      FeatureType::Battery => Ok(SensorType::Battery),
      FeatureType::RSSI => Ok(SensorType::RSSI),
      FeatureType::Button => Ok(SensorType::Button),
      FeatureType::Pressure => Ok(SensorType::Pressure),
      _ => Err(format!(
        "Feature type {value} not valid for SensorType conversion"
      )),
    }
  }
}


impl From<ActuatorType> for FeatureType {
  fn from(value: ActuatorType) -> Self {
    match value {
      ActuatorType::Unknown => FeatureType::Unknown,
      ActuatorType::Vibrate => FeatureType::Vibrate,
      ActuatorType::Rotate => FeatureType::Rotate,
      ActuatorType::Heater => FeatureType::Heater,
      ActuatorType::Led => FeatureType::Led,
      ActuatorType::RotateWithDirection => FeatureType::RotateWithDirection,
      ActuatorType::PositionWithDuration => FeatureType::PositionWithDuration,
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
  Clone, Debug, Default, PartialEq, Getters, MutGetters, Setters, Serialize, Deserialize,
)]
pub struct DeviceFeature {
  // Index of the feature on the device. This was originally implicit as the position in the feature
  // array. We now make it explicit even though it's still just array position, because implicit
  // array positions have made life hell in so many different ways.
  #[getset(get = "pub")]
  #[serde(rename="FeatureIndex")]
  feature_index: u32,
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(default)]
  #[serde(rename="FeatureDescription")]
  description: String,
  #[getset(get = "pub")]
  #[serde(rename = "FeatureType")]
  feature_type: FeatureType,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "Actuator")]
  actuator: Option<HashMap<ActuatorType, DeviceFeatureActuator>>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "Sensor")]
  sensor: Option<HashMap<SensorType, DeviceFeatureSensor>>,
  #[getset(get = "pub")]
  #[serde(rename = "Raw")]
  #[serde(skip_serializing_if="Option::is_none")]
  raw: Option<DeviceFeatureRaw>,
}

impl DeviceFeature {
  pub fn new(
    index: u32,
    description: &str,
    feature_type: FeatureType,
    actuator: &Option<HashMap<ActuatorType, DeviceFeatureActuator>>,
    sensor: &Option<HashMap<SensorType, DeviceFeatureSensor>>,
    raw: &Option<DeviceFeatureRaw>,
  ) -> Self {
    Self {
      feature_index: index,
      description: description.to_owned(),
      feature_type,
      actuator: actuator.clone(),
      sensor: sensor.clone(),
      raw: raw.clone(),
    }
  }

  pub fn new_raw_feature(index: u32, endpoints: &[Endpoint]) -> Self {
    Self {
      feature_index: index,
      description: "Raw Endpoints".to_owned(),
      feature_type: FeatureType::Raw,
      actuator: None,
      sensor: None,
      raw: Some(DeviceFeatureRaw::new(endpoints)),
    }
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

#[derive(Clone, Debug, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize)]
pub struct DeviceFeatureActuator {
  #[getset(get = "pub")]
  #[serde(rename = "StepCount")]
  step_count: u32,
}

impl DeviceFeatureActuator {
  pub fn new(
    step_count: u32,
  ) -> Self {
    Self {
      step_count,
    }
  }
}

#[derive(
  Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize,
)]
pub struct DeviceFeatureSensor {
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "ValueRange")]
  #[serde(serialize_with = "range_sequence_serialize")]
  value_range: Vec<RangeInclusive<i32>>,
  #[getset(get = "pub")]
  #[serde(rename = "SensorCommands")]
  sensor_commands: HashSet<SensorCommandType>,
}

impl DeviceFeatureSensor {
  pub fn new(
    value_range: &Vec<RangeInclusive<i32>>,
    sensor_commands: &HashSet<SensorCommandType>,
  ) -> Self {
    Self {
      value_range: value_range.clone(),
      sensor_commands: sensor_commands.clone(),
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
}

impl DeviceFeatureRaw {
  pub fn new(endpoints: &[Endpoint]) -> Self {
    Self {
      endpoints: endpoints.into(),
    }
  }
}
