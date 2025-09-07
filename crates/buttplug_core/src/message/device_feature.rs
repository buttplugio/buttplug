// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{message::InputCommandType, util::range_serialize::*};
use derive_builder::Builder;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
use std::{
  collections::HashSet, hash::Hash, ops::RangeInclusive
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
#[serde(rename_all="PascalCase")]
pub struct DeviceFeature {
  // Index of the feature on the device. This was originally implicit as the position in the feature
  // array. We now make it explicit even though it's still just array position, because implicit
  // array positions have made life hell in so many different ways.
  #[getset(get_copy = "pub")]
  feature_index: u32,
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(default, rename = "FeatureDescription")]
  description: String,
  // TODO Maybe make this its own object instead of a HashMap?
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  output: Option<DeviceFeatureOutput>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  input: Option<DeviceFeatureInput>,
}

impl DeviceFeature {
  pub fn new(
    index: u32,
    description: &str,
    output: &Option<DeviceFeatureOutput>,
    input: &Option<DeviceFeatureInput>,
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

pub trait DeviceFeatureOutputLimits {
  fn step_count(&self) -> u32;
  fn step_limit(&self) -> RangeInclusive<i32>;
}

#[derive(Serialize, Deserialize, Clone, Debug, Getters)]
#[serde(rename_all="PascalCase")]
pub struct DeviceFeatureOutputValueProperties {
  #[getset(get = "pub")]
  #[serde(serialize_with = "range_serialize")]
  value: RangeInclusive<i32>,
}

impl DeviceFeatureOutputValueProperties {
  pub fn new(value: &RangeInclusive<i32>) -> Self {
    DeviceFeatureOutputValueProperties { value: value.clone() }
  }

  pub fn step_count(&self) -> u32 {
    *self.value.end() as u32
  }
}

impl DeviceFeatureOutputLimits for DeviceFeatureOutputValueProperties {
  fn step_count(&self) -> u32 {
    self.step_count()
  }
  fn step_limit(&self) -> RangeInclusive<i32> {
    self.value.clone()
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, Getters)]
#[serde(rename_all="PascalCase")]
pub struct DeviceFeatureOutputPositionWithDurationProperties {
  #[getset(get = "pub")]
  #[serde(serialize_with = "range_serialize")]
  position: RangeInclusive<i32>,
  #[getset(get = "pub")]
  #[serde(serialize_with = "range_serialize")]
  duration: RangeInclusive<i32>,
}

impl DeviceFeatureOutputPositionWithDurationProperties {
  pub fn new(position: &RangeInclusive<i32>, duration: &RangeInclusive<i32>) -> Self {
    DeviceFeatureOutputPositionWithDurationProperties { position: position.clone(), duration: duration.clone() }
  }

  pub fn step_count(&self) -> u32 {
    *self.position.end() as u32
  }
}

impl DeviceFeatureOutputLimits for DeviceFeatureOutputPositionWithDurationProperties {
  fn step_count(&self) -> u32 {
    self.step_count()
  }
  fn step_limit(&self) -> RangeInclusive<i32> {
    self.position.clone()
  }
}

#[derive(Clone, Debug, Getters, Setters, Default, Serialize, Deserialize, Builder)]
#[builder(setter(strip_option), default)]
#[getset(get = "pub")]
#[serde(rename_all="PascalCase")]
pub struct DeviceFeatureOutput {
  #[serde(skip_serializing_if="Option::is_none")]
  vibrate: Option<DeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  rotate: Option<DeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  rotate_with_direction: Option<DeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  oscillate: Option<DeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  constrict: Option<DeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  heater: Option<DeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  led: Option<DeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  position: Option<DeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  position_with_duration: Option<DeviceFeatureOutputPositionWithDurationProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  spray: Option<DeviceFeatureOutputValueProperties>,
}

impl DeviceFeatureOutput {
  pub fn contains(&self, output_type: OutputType) -> bool {
    match output_type {
      OutputType::Constrict => self.constrict.is_some(),
      OutputType::Heater => self.heater.is_some(),
      OutputType::Led => self.led.is_some(),
      OutputType::Oscillate => self.oscillate.is_some(),
      OutputType::Position => self.position.is_some(),
      OutputType::PositionWithDuration => self.position_with_duration.is_some(),
      OutputType::Rotate => self.rotate.is_some(),
      OutputType::RotateWithDirection => self.rotate_with_direction.is_some(),
      OutputType::Spray => self.spray.is_some(),
      OutputType::Unknown => false,
      OutputType::Vibrate => self.vibrate.is_some(),
    }
  }

  pub fn get(&self, output_type: OutputType) -> Option<&dyn DeviceFeatureOutputLimits> {
    match output_type {
      OutputType::Constrict => self.constrict().as_ref().map(|x| x as &dyn DeviceFeatureOutputLimits),
      OutputType::Heater => self.heater().as_ref().map(|x| x as &dyn DeviceFeatureOutputLimits),
      OutputType::Led => self.led().as_ref().map(|x| x as &dyn DeviceFeatureOutputLimits),
      OutputType::Oscillate => self.oscillate().as_ref().map(|x| x as &dyn DeviceFeatureOutputLimits),
      OutputType::Position => self.position().as_ref().map(|x| x as &dyn DeviceFeatureOutputLimits),
      OutputType::PositionWithDuration => self.position_with_duration().as_ref().map(|x| x as &dyn DeviceFeatureOutputLimits),
      OutputType::Rotate => self.rotate().as_ref().map(|x| x as &dyn DeviceFeatureOutputLimits),
      OutputType::RotateWithDirection => self.rotate_with_direction().as_ref().map(|x| x as &dyn DeviceFeatureOutputLimits),
      OutputType::Spray => self.spray().as_ref().map(|x| x as &dyn DeviceFeatureOutputLimits),
      OutputType::Unknown => None,
      OutputType::Vibrate => self.vibrate().as_ref().map(|x| x as &dyn DeviceFeatureOutputLimits),
    }
  }
}

#[derive(
  Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize,
)]
#[serde(rename_all="PascalCase")]
pub struct DeviceFeatureInputProperties {
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(serialize_with = "range_sequence_serialize")]
  value_range: Vec<RangeInclusive<i32>>,
  #[getset(get = "pub")]
  input_commands: HashSet<InputCommandType>,
}

impl DeviceFeatureInputProperties {
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


#[derive(Clone, Debug, Getters, Setters, Default, Serialize, Deserialize, Builder)]
#[builder(setter(strip_option), default)]
#[getset(get = "pub")]
#[serde(rename_all="PascalCase")]
pub struct DeviceFeatureInput {
  #[serde(skip_serializing_if="Option::is_none")]
  battery: Option<DeviceFeatureInputProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  rssi: Option<DeviceFeatureInputProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  pressure: Option<DeviceFeatureInputProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  button: Option<DeviceFeatureInputProperties>,
}

impl DeviceFeatureInput {
  pub fn contains(&self, input_type: InputType) -> bool {
    match input_type {
      InputType::Battery => self.battery.is_some(),
      InputType::Rssi => self.rssi.is_some(),
      InputType::Pressure => self.pressure.is_some(),
      InputType::Button => self.button.is_some(),
      InputType::Unknown => false,
    }
  }

  pub fn get(&self, input_type: InputType) -> &Option<DeviceFeatureInputProperties> {
    match input_type {
      InputType::Battery => self.battery(),
      InputType::Rssi => self.rssi(),
      InputType::Pressure => self.pressure(),
      InputType::Button => self.button(),
      InputType::Unknown => &None,
    }
  }
}