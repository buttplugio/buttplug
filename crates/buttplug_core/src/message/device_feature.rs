// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  message::InputCommandType,
  util::{
    range::RangeInclusive,
    small_vec_enum_map::{SmallVecEnumMap, VariantKey},
  },
};
use enumflags2::BitFlags;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};

// This will look almost exactly like ServerDeviceFeature. However, it will only contain
// information we want the client to know, i.e. step counts versus specific step ranges. This is
// what will be sent to the client as part of DeviceAdded/DeviceList messages. It should not be used
// for outside configuration/serialization, rather it should be a subset of that information.
//
// For many messages, client and server configurations may be exactly the same. If they are not,
// then we denote this by prefixing the type with Client/Server. Server attributes will usually be
// hosted in the server/device/configuration module.
//
// SERIALIZATION NOTE: This type and its children use PascalCase field names because they are part
// of the wire protocol (DeviceAdded/DeviceList messages). Internal server-side types
// (ServerDeviceFeature and friends) use snake_case. Never use DeviceFeature for internal
// storage or config files.
#[derive(
  Clone, Debug, Default, Getters, MutGetters, CopyGetters, Setters, Serialize, Deserialize,
)]
#[serde(rename_all = "PascalCase")]
pub struct DeviceFeature {
  // Index of the feature on the device. This was originally implicit as the position in the feature
  // array. We now make it explicit even though it's still just array position, because implicit
  // array positions have made life hell in so many different ways.
  #[getset(get_copy = "pub")]
  feature_index: u32,
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(default, rename = "FeatureDescription")]
  description: String,
  #[serde(skip_serializing_if = "SmallVecEnumMap::is_empty")]
  output: SmallVecEnumMap<DeviceFeatureOutput, 1>,
  #[serde(skip_serializing_if = "SmallVecEnumMap::is_empty")]
  input: SmallVecEnumMap<DeviceFeatureInput, 1>,
}

impl DeviceFeature {
  pub fn new(
    index: u32,
    description: &str,
    output: &SmallVecEnumMap<DeviceFeatureOutput, 1>,
    input: &SmallVecEnumMap<DeviceFeatureInput, 1>,
  ) -> Self {
    Self {
      feature_index: index,
      description: description.to_owned(),
      output: output.clone(),
      input: input.clone(),
    }
  }

  pub fn contains_output(&self, output_type: OutputType) -> bool {
    self.output.contains_key(&output_type)
  }

  pub fn contains_input(&self, input_type: InputType) -> bool {
    self.input.contains_key(&input_type)
  }

  pub fn get_output(&self, output_type: OutputType) -> Option<&DeviceFeatureOutput> {
    self.output.find_by_key(&output_type)
  }

  pub fn get_output_limits(
    &self,
    output_type: OutputType,
  ) -> Option<&dyn DeviceFeatureOutputLimits> {
    self.output.find_by_key(&output_type).map(|o| o.as_limits())
  }

  pub fn get_input(&self, input_type: InputType) -> Option<&DeviceFeatureInput> {
    self.input.find_by_key(&input_type)
  }
}

pub trait DeviceFeatureOutputLimits {
  fn step_count(&self) -> u32;
  fn step_limit(&self) -> RangeInclusive<i32>;
}

#[derive(Serialize, Deserialize, Clone, Debug, Getters)]
#[serde(rename_all = "PascalCase")]
pub struct DeviceFeatureOutputValueProperties {
  #[getset(get = "pub")]
  value: RangeInclusive<i32>,
}

impl DeviceFeatureOutputValueProperties {
  pub fn new(value: RangeInclusive<i32>) -> Self {
    DeviceFeatureOutputValueProperties { value }
  }
}

impl DeviceFeatureOutputLimits for DeviceFeatureOutputValueProperties {
  fn step_count(&self) -> u32 {
    self.value.end() as u32
  }
  fn step_limit(&self) -> RangeInclusive<i32> {
    self.value
  }
}

#[derive(Serialize, Deserialize, Clone, Debug, Getters)]
#[serde(rename_all = "PascalCase")]
pub struct DeviceFeatureOutputHwPositionWithDurationProperties {
  #[getset(get = "pub")]
  value: RangeInclusive<i32>,
  #[getset(get = "pub")]
  duration: RangeInclusive<i32>,
}

impl DeviceFeatureOutputHwPositionWithDurationProperties {
  pub fn new(position: RangeInclusive<i32>, duration: RangeInclusive<i32>) -> Self {
    DeviceFeatureOutputHwPositionWithDurationProperties {
      value: position,
      duration,
    }
  }
}

impl DeviceFeatureOutputLimits for DeviceFeatureOutputHwPositionWithDurationProperties {
  fn step_count(&self) -> u32 {
    self.value.end() as u32
  }
  fn step_limit(&self) -> RangeInclusive<i32> {
    self.value
  }
}

// OutputType is auto-generated as the discriminant enum of DeviceFeatureOutput.
// Adding or renaming a DeviceFeatureOutput variant automatically updates OutputType.
#[derive(Clone, Debug, Serialize, Deserialize, EnumDiscriminants)]
#[strum_discriminants(name(OutputType))]
#[strum_discriminants(vis(pub))]
#[strum_discriminants(derive(Display, Hash, EnumIter, EnumString, Serialize, Deserialize))]
pub enum DeviceFeatureOutput {
  Vibrate(DeviceFeatureOutputValueProperties),
  Rotate(DeviceFeatureOutputValueProperties),
  Oscillate(DeviceFeatureOutputValueProperties),
  Constrict(DeviceFeatureOutputValueProperties),
  Temperature(DeviceFeatureOutputValueProperties),
  Led(DeviceFeatureOutputValueProperties),
  Position(DeviceFeatureOutputValueProperties),
  HwPositionWithDuration(DeviceFeatureOutputHwPositionWithDurationProperties),
  Spray(DeviceFeatureOutputValueProperties),
}

impl DeviceFeatureOutput {
  pub fn output_type(&self) -> OutputType {
    OutputType::from(self)
  }

  fn as_limits(&self) -> &dyn DeviceFeatureOutputLimits {
    match self {
      DeviceFeatureOutput::Vibrate(v) => v,
      DeviceFeatureOutput::Rotate(v) => v,
      DeviceFeatureOutput::Oscillate(v) => v,
      DeviceFeatureOutput::Constrict(v) => v,
      DeviceFeatureOutput::Temperature(v) => v,
      DeviceFeatureOutput::Led(v) => v,
      DeviceFeatureOutput::Position(v) => v,
      DeviceFeatureOutput::HwPositionWithDuration(v) => v,
      DeviceFeatureOutput::Spray(v) => v,
    }
  }
}

impl VariantKey for DeviceFeatureOutput {
  type Key = OutputType;
  fn variant_key(&self) -> OutputType {
    self.output_type()
  }
}

#[derive(
  Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize,
)]
#[serde(rename_all = "PascalCase")]
pub struct DeviceFeatureInputProperties {
  #[getset(get = "pub", get_mut = "pub(super)")]
  value: Vec<RangeInclusive<i32>>,
  #[getset(get = "pub")]
  #[serde(with = "crate::util::serializers::bitflags_seq")]
  command: BitFlags<InputCommandType>,
}

impl DeviceFeatureInputProperties {
  pub fn new(
    value: &Vec<RangeInclusive<i32>>,
    sensor_commands: &BitFlags<InputCommandType>,
  ) -> Self {
    Self {
      value: value.clone(),
      command: sensor_commands.clone(),
    }
  }
}

// InputType is auto-generated as the discriminant enum of DeviceFeatureInput.
#[derive(Clone, Debug, Serialize, Deserialize, EnumDiscriminants)]
#[strum_discriminants(name(InputType))]
#[strum_discriminants(vis(pub))]
#[strum_discriminants(derive(Display, Hash, EnumIter, EnumString, Serialize, Deserialize))]
pub enum DeviceFeatureInput {
  Battery(DeviceFeatureInputProperties),
  Rssi(DeviceFeatureInputProperties),
  Button(DeviceFeatureInputProperties),
  Pressure(DeviceFeatureInputProperties),
  Depth(DeviceFeatureInputProperties),
  Position(DeviceFeatureInputProperties),
}

impl DeviceFeatureInput {
  pub fn input_type(&self) -> InputType {
    InputType::from(self)
  }

  pub fn command(&self) -> &BitFlags<InputCommandType> {
    match self {
      DeviceFeatureInput::Battery(p) => p.command(),
      DeviceFeatureInput::Rssi(p) => p.command(),
      DeviceFeatureInput::Button(p) => p.command(),
      DeviceFeatureInput::Pressure(p) => p.command(),
      DeviceFeatureInput::Depth(p) => p.command(),
      DeviceFeatureInput::Position(p) => p.command(),
    }
  }
}

impl VariantKey for DeviceFeatureInput {
  type Key = InputType;
  fn variant_key(&self) -> InputType {
    self.input_type()
  }
}
