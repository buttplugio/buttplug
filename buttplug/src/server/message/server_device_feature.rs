// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::{
  errors::ButtplugDeviceError,
  message::{
    OutputType,
    DeviceFeature,
    DeviceFeatureOutput,
    DeviceFeatureRaw,
    DeviceFeatureInput,
    Endpoint,
    FeatureType,
    InputCommandType,
    InputType,
  },
};
use getset::{CopyGetters, Getters, MutGetters, Setters};
use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
use std::{
  collections::{HashMap, HashSet},
  ops::RangeInclusive,
};
use uuid::Uuid;

// This will look almost exactly like ServerDeviceFeature. However, it will only contain
// information we want the client to know, i.e. step counts versus specific step ranges. This is
// what will be sent to the client as part of DeviceAdded/DeviceList messages. It should not be used
// for outside configuration/serialization, rather it should be a subset of that information.
//
// For many messages, client and server configurations may be exactly the same. If they are not,
// then we denote this by prefixing the type with Client/Server. Server attributes will usually be
// hosted in the server/device/configuration module.
#[derive(
  Clone,
  Debug,
  Default,
  PartialEq,
  Eq,
  Getters,
  MutGetters,
  Setters,
  Serialize,
  Deserialize,
  CopyGetters,
)]
pub struct ServerDeviceFeature {
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(default)]
  description: String,
  #[getset(get_copy = "pub")]
  #[serde(rename = "feature-type")]
  feature_type: FeatureType,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "output")]
  output: Option<HashMap<OutputType, ServerDeviceFeatureOutput>>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "input")]
  input: Option<HashMap<InputType, ServerDeviceFeatureInput>>,
  #[getset(get = "pub")]
  #[serde(skip)]
  raw: Option<DeviceFeatureRaw>,
  #[getset(get_copy = "pub", get_mut = "pub(super)")]
  id: Uuid,
  #[getset(get_copy = "pub", get_mut = "pub(super)")]
  #[serde(rename = "base-id", skip_serializing_if = "Option::is_none")]
  base_id: Option<Uuid>,
}

impl ServerDeviceFeature {
  pub fn new(
    description: &str,
    id: &Uuid,
    base_id: &Option<Uuid>,
    feature_type: FeatureType,
    output: &Option<HashMap<OutputType, ServerDeviceFeatureOutput>>,
    input: &Option<HashMap<InputType, ServerDeviceFeatureInput>>,
  ) -> Self {
    Self {
      description: description.to_owned(),
      feature_type,
      output: output.clone(),
      input: input.clone(),
      raw: None,
      id: *id,
      base_id: *base_id,
    }
  }

  pub fn is_valid(&self) -> Result<(), ButtplugDeviceError> {
    if let Some(output_map) = &self.output {
      for actuator in output_map.values() {
        actuator.is_valid()?;
      }
    }
    Ok(())
  }

  pub fn as_device_feature(&self, index: u32) -> DeviceFeature {
    DeviceFeature::new(
      index,
      self.description(),
      self.feature_type(),
      &self.output.clone().map(|x| {
        x.iter()
          .map(|(t, a)| (*t, DeviceFeatureOutput::from(a.clone())))
          .collect()
      }),
      &self.input.clone().map(|x| {
        x.iter()
          .map(|(t, a)| (*t, DeviceFeatureInput::from(a.clone())))
          .collect()
      }),
      self.raw(),
    )
  }

  /// If this is a base feature (i.e. base_id is None), create a new feature with a randomized id
  /// and the current feature id as the base id. Otherwise, just pass back a copy of self.
  pub fn as_user_feature(&self) -> Self {
    if self.base_id.is_some() {
      self.clone()
    } else {
      Self {
        description: self.description.clone(),
        feature_type: self.feature_type,
        output: self.output.clone(),
        input: self.input.clone(),
        raw: self.raw.clone(),
        id: Uuid::new_v4(),
        base_id: Some(self.id),
      }
    }
  }

  pub fn new_raw_feature(endpoints: &[Endpoint]) -> Self {
    Self {
      description: "Raw Endpoints".to_owned(),
      feature_type: FeatureType::Raw,
      output: None,
      input: None,
      raw: Some(DeviceFeatureRaw::new(endpoints)),
      id: uuid::Uuid::new_v4(),
      base_id: None,
    }
  }
}

fn range_serialize<S>(range: &RangeInclusive<u32>, serializer: S) -> Result<S::Ok, S::Error>
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

// Copy class used for deserialization, so we can have an optional step-limit
#[derive(Clone, Debug, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize)]
pub struct ServerDeviceFeatureActuatorSerialized {
  #[getset(get = "pub")]
  #[serde(rename = "step-range")]
  #[serde(serialize_with = "range_serialize")]
  step_range: RangeInclusive<u32>,
  // This doesn't exist in base configs, so when we load these from the base config file, we'll just
  // copy the step_range value.
  #[getset(get = "pub")]
  #[serde(rename = "step-limit")]
  #[serde(default)]
  step_limit: Option<RangeInclusive<u32>>,
}

impl From<ServerDeviceFeatureActuatorSerialized> for ServerDeviceFeatureOutput {
  fn from(value: ServerDeviceFeatureActuatorSerialized) -> Self {
    Self {
      step_range: value.step_range.clone(),
      step_limit: value.step_limit.unwrap_or(value.step_range.clone()),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize)]
#[serde(from = "ServerDeviceFeatureActuatorSerialized")]
pub struct ServerDeviceFeatureOutput {
  #[getset(get = "pub")]
  #[serde(rename = "step-range")]
  #[serde(serialize_with = "range_serialize")]
  step_range: RangeInclusive<u32>,
  // This doesn't exist in base configs, so when we load these from the base config file, we'll just
  // copy the step_range value.
  #[getset(get = "pub")]
  #[serde(rename = "step-limit")]
  #[serde(serialize_with = "range_serialize")]
  step_limit: RangeInclusive<u32>,
}

impl ServerDeviceFeatureOutput {
  pub fn new(step_range: &RangeInclusive<u32>, step_limit: &RangeInclusive<u32>) -> Self {
    Self {
      step_range: step_range.clone(),
      step_limit: step_limit.clone(),
    }
  }

  pub fn step_count(&self) -> u32 {
    self.step_limit.end() - self.step_limit().start()
  }

  pub fn is_valid(&self) -> Result<(), ButtplugDeviceError> {
    if self.step_range.is_empty() {
      Err(ButtplugDeviceError::DeviceConfigurationError(
        "Step range empty.".to_string(),
      ))
    } else if self.step_limit.is_empty() {
      Err(ButtplugDeviceError::DeviceConfigurationError(
        "Step limit empty.".to_string(),
      ))
    } else {
      Ok(())
    }
  }
}

impl From<ServerDeviceFeatureOutput> for DeviceFeatureOutput {
  fn from(value: ServerDeviceFeatureOutput) -> Self {
    DeviceFeatureOutput::new(value.step_limit().end() - value.step_limit().start())
  }
}

#[derive(
  Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize,
)]
pub struct ServerDeviceFeatureInput {
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "value-range")]
  #[serde(serialize_with = "range_sequence_serialize")]
  value_range: Vec<RangeInclusive<i32>>,
  #[getset(get = "pub")]
  #[serde(rename = "input-commands")]
  input_commands: HashSet<InputCommandType>,
}

impl ServerDeviceFeatureInput {
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

impl From<ServerDeviceFeatureInput> for DeviceFeatureInput {
  fn from(value: ServerDeviceFeatureInput) -> Self {
    // Unlike actuator, this is just a straight copy.
    DeviceFeatureInput::new(value.value_range(), value.input_commands())
  }
}
