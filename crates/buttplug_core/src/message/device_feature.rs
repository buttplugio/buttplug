// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::InputCommandType;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
use std::{
  collections::{HashMap, HashSet},
  ops::RangeInclusive,
};

#[derive(Debug, Default, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter)]
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
  Spray,
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
  Rssi,
  Button,
  Pressure,
  // Currently unused but possible sensor features:
  // Temperature,
  // Accelerometer,
  // Gyro,
  //
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, EnumIter)]
pub enum OutputType {
  Unknown,
  Vibrate,
  // Single Direction Rotation Speed
  Rotate,
  // Two Direction Rotation Speed
  RotateWithDirection,
  Oscillate,
  Constrict,
  Heater,
  Led,
  // For instances where we specify a position to move to ASAP. Usually servos, probably for the
  // OSR-2/SR-6.
  Position,
  PositionWithDuration,
  // Lube shooters
  Spray,
  // Things we might add in the future
  // Inflate,
}

impl TryFrom<FeatureType> for OutputType {
  type Error = String;
  fn try_from(value: FeatureType) -> Result<Self, Self::Error> {
    match value {
      FeatureType::Unknown => Ok(OutputType::Unknown),
      FeatureType::Vibrate => Ok(OutputType::Vibrate),
      FeatureType::Rotate => Ok(OutputType::Rotate),
      FeatureType::Heater => Ok(OutputType::Heater),
      FeatureType::Led => Ok(OutputType::Led),
      FeatureType::RotateWithDirection => Ok(OutputType::RotateWithDirection),
      FeatureType::PositionWithDuration => Ok(OutputType::PositionWithDuration),
      FeatureType::Oscillate => Ok(OutputType::Oscillate),
      FeatureType::Constrict => Ok(OutputType::Constrict),
      FeatureType::Spray => Ok(OutputType::Spray),
      FeatureType::Position => Ok(OutputType::Position),
      _ => Err(format!(
        "Feature type {value} not valid for OutputType conversion"
      )),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, Hash, EnumIter)]
pub enum InputType {
  Unknown,
  Battery,
  Rssi,
  Button,
  Pressure,
  // Temperature,
  // Accelerometer,
  // Gyro,
}

impl TryFrom<FeatureType> for InputType {
  type Error = String;
  fn try_from(value: FeatureType) -> Result<Self, Self::Error> {
    match value {
      FeatureType::Unknown => Ok(InputType::Unknown),
      FeatureType::Battery => Ok(InputType::Battery),
      FeatureType::Rssi => Ok(InputType::Rssi),
      FeatureType::Button => Ok(InputType::Button),
      FeatureType::Pressure => Ok(InputType::Pressure),
      _ => Err(format!(
        "Feature type {value} not valid for SensorType conversion"
      )),
    }
  }
}

impl From<OutputType> for FeatureType {
  fn from(value: OutputType) -> Self {
    match value {
      OutputType::Unknown => FeatureType::Unknown,
      OutputType::Vibrate => FeatureType::Vibrate,
      OutputType::Rotate => FeatureType::Rotate,
      OutputType::Heater => FeatureType::Heater,
      OutputType::Led => FeatureType::Led,
      OutputType::RotateWithDirection => FeatureType::RotateWithDirection,
      OutputType::PositionWithDuration => FeatureType::PositionWithDuration,
      OutputType::Oscillate => FeatureType::Oscillate,
      OutputType::Constrict => FeatureType::Constrict,
      OutputType::Spray => FeatureType::Spray,
      OutputType::Position => FeatureType::Position,
    }
  }
}

impl From<InputType> for FeatureType {
  fn from(value: InputType) -> Self {
    match value {
      InputType::Unknown => FeatureType::Unknown,
      InputType::Battery => FeatureType::Battery,
      InputType::Rssi => FeatureType::Rssi,
      InputType::Button => FeatureType::Button,
      InputType::Pressure => FeatureType::Pressure,
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
  Clone, Debug, Default, PartialEq, Getters, MutGetters, CopyGetters, Setters, Serialize, Deserialize,
)]
pub struct DeviceFeature {
  // Index of the feature on the device. This was originally implicit as the position in the feature
  // array. We now make it explicit even though it's still just array position, because implicit
  // array positions have made life hell in so many different ways.
  #[getset(get_copy = "pub")]
  #[serde(rename = "FeatureIndex")]
  feature_index: u32,
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(default)]
  #[serde(rename = "FeatureDescription")]
  description: String,
  #[getset(get_copy = "pub")]
  #[serde(rename = "FeatureType")]
  feature_type: FeatureType,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "Output")]
  output: Option<HashMap<OutputType, DeviceFeatureOutput>>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "Input")]
  input: Option<HashMap<InputType, DeviceFeatureInput>>,
}

impl DeviceFeature {
  pub fn new(
    index: u32,
    description: &str,
    feature_type: FeatureType,
    output: &Option<HashMap<OutputType, DeviceFeatureOutput>>,
    input: &Option<HashMap<InputType, DeviceFeatureInput>>,
  ) -> Self {
    Self {
      feature_index: index,
      description: description.to_owned(),
      feature_type,
      output: output.clone(),
      input: input.clone(),
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

#[derive(Clone, Debug, PartialEq, Eq, CopyGetters, Serialize, Deserialize)]
pub struct DeviceFeatureOutput {
  #[getset(get_copy = "pub")]
  #[serde(rename = "StepCount")]
  step_count: u32,
}

impl DeviceFeatureOutput {
  pub fn new(step_count: u32) -> Self {
    Self { step_count }
  }
}

#[derive(
  Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize,
)]
pub struct DeviceFeatureInput {
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "ValueRange")]
  #[serde(serialize_with = "range_sequence_serialize")]
  value_range: Vec<RangeInclusive<i32>>,
  #[getset(get = "pub")]
  #[serde(rename = "InputCommands")]
  input_commands: HashSet<InputCommandType>,
}

impl DeviceFeatureInput {
  pub fn new(
    value_range: &Vec<RangeInclusive<i32>>,
    sensor_commands: &HashSet<InputCommandType>,
  ) -> Self {
    Self {
      value_range: value_range.clone(),
      input_commands: sensor_commands.clone(),
    }
  }
}
