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

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, EnumIter)]
pub enum OutputType {
  Unknown,
  #[serde(alias = "vibrate")]
  Vibrate,
  // Single Direction Rotation Speed
  #[serde(alias = "rotate")]
  Rotate,
  // Two Direction Rotation Speed
  #[serde(alias = "rotate_with_direction")]
  RotateWithDirection,
  #[serde(alias = "oscillate")]
  Oscillate,
  #[serde(alias = "constrict")]
  Constrict,
  #[serde(alias = "heater")]
  Heater,
  #[serde(alias = "led")]
  Led,
  // For instances where we specify a position to move to ASAP. Usually servos, probably for the
  // OSR-2/SR-6.
  #[serde(alias = "position")]
  Position,
  #[serde(alias = "position_with_duration")]
  PositionWithDuration,
  // Lube shooters
  #[serde(alias = "spray")]
  Spray,
  // Things we might add in the future
  // Inflate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, Hash, EnumIter)]
pub enum InputType {
  Unknown,
  #[serde(alias = "battery")]
  Battery,
  #[serde(alias = "rssi")]
  Rssi,
  #[serde(alias = "button")]
  Button,
  #[serde(alias = "pressure")]
  Pressure,
  // Temperature,
  // Accelerometer,
  // Gyro,
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
    output: &Option<HashMap<OutputType, DeviceFeatureOutput>>,
    input: &Option<HashMap<InputType, DeviceFeatureInput>>,
  ) -> Self {
    Self {
      feature_index: index,
      description: description.to_owned(),
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
