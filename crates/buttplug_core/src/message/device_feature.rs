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
  collections::{HashMap, HashSet}, hash::Hash, ops::RangeInclusive
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
  Clone, Debug, Default, Getters, MutGetters, CopyGetters, Setters, Serialize, Deserialize,
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
  // TODO Maybe make this its own object instead of a HashMap?
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "Output")]
  output: Option<HashSet<DeviceFeatureOutput>>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "Input")]
  input: Option<HashMap<InputType, DeviceFeatureInput>>,
}

impl DeviceFeature {
  pub fn new(
    index: u32,
    description: &str,
    output: &Option<HashSet<DeviceFeatureOutput>>,
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

#[derive(Serialize, Deserialize, Clone, Debug, Getters)]
pub struct DeviceFeatureOutputValueProperties {
  #[getset(get = "pub")]
  #[serde(rename = "Value")]
  value: RangeInclusive<i32>,
}

impl DeviceFeatureOutputValueProperties {
  pub fn new(value: &RangeInclusive<i32>) -> Self {
    DeviceFeatureOutputValueProperties { value: value.clone() }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, Getters)]
pub struct DeviceFeatureOutputPositionWithDurationProperties {
  #[getset(get = "pub")]
  #[serde(rename = "Position")]
  position: RangeInclusive<u32>,
  #[getset(get = "pub")]
  #[serde(rename = "Duration")]
  duration: RangeInclusive<u32>,
}

impl DeviceFeatureOutputPositionWithDurationProperties {
  pub fn new(position: &RangeInclusive<u32>, duration: &RangeInclusive<u32>) -> Self {
    DeviceFeatureOutputPositionWithDurationProperties { position: position.clone(), duration: duration.clone() }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DeviceFeatureOutput {
  Unknown,
  Vibrate(DeviceFeatureOutputValueProperties),
  Rotate(DeviceFeatureOutputValueProperties),
  RotateWithDirection(DeviceFeatureOutputValueProperties),
  Oscillate(DeviceFeatureOutputValueProperties),
  Constrict(DeviceFeatureOutputValueProperties),
  Heater(DeviceFeatureOutputValueProperties),
  Led(DeviceFeatureOutputValueProperties),
  Position(DeviceFeatureOutputValueProperties),
  PositionWithDuration(DeviceFeatureOutputPositionWithDurationProperties),
  Spray(DeviceFeatureOutputValueProperties),
}

impl From<&DeviceFeatureOutput> for OutputType {
  fn from(value: &DeviceFeatureOutput) -> Self {
    match value {
      DeviceFeatureOutput::Constrict(_) => OutputType::Constrict,
      DeviceFeatureOutput::Heater(_) => OutputType::Heater,
      DeviceFeatureOutput::Led(_) => OutputType::Led,
      DeviceFeatureOutput::Oscillate(_) => OutputType::Oscillate,
      DeviceFeatureOutput::Position(_) => OutputType::Position,
      DeviceFeatureOutput::PositionWithDuration(_) => OutputType::PositionWithDuration,
      DeviceFeatureOutput::Rotate(_) => OutputType::Rotate,
      DeviceFeatureOutput::RotateWithDirection(_) => OutputType::RotateWithDirection,
      DeviceFeatureOutput::Spray(_) => OutputType::Spray,
      DeviceFeatureOutput::Unknown => OutputType::Unknown,
      DeviceFeatureOutput::Vibrate(_) => OutputType::Vibrate
    }
  }
}

impl PartialEq for DeviceFeatureOutput {
  fn eq(&self, other: &Self) -> bool {
    // Just make sure our two DeviceFeatureOutput's are the same variant, their values may not match
    // but we should never store two of the same variant in the same structure.
    std::mem::discriminant(self) == std::mem::discriminant(other)
  }
}

impl Eq for DeviceFeatureOutput {}

impl Hash for DeviceFeatureOutput {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    OutputType::from(self).hash(state)
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
