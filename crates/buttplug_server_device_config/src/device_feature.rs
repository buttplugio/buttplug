// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::BaseFeatureSettings;
use buttplug_core::{
  errors::ButtplugDeviceError,
  message::{
    DeviceFeature,
    DeviceFeatureInput,
    DeviceFeatureOutput,
    FeatureType,
    InputCommandType,
    InputType,
    OutputType,
  },
};
use getset::{CopyGetters, Getters, MutGetters, Setters};
use serde::{
  ser::{self, SerializeSeq},
  Deserialize,
  Serialize,
  Serializer,
};
use std::{
  collections::{HashMap, HashSet},
  ops::RangeInclusive,
};
use uuid::Uuid;

fn range_serialize<S>(range: &Option<RangeInclusive<u32>>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  if let Some(range) = range {
    let mut seq = serializer.serialize_seq(Some(2))?;
    seq.serialize_element(&range.start())?;
    seq.serialize_element(&range.end())?;
    seq.end()
  } else {
    Err(ser::Error::custom(
      "shouldn't be serializing if range is None",
    ))
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

#[derive(
  Clone, Debug, Default, Getters, MutGetters, Setters, Serialize, Deserialize, CopyGetters,
)]
pub struct ServerBaseDeviceFeature {
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(default)]
  description: String,
  #[getset(get_copy = "pub")]
  #[serde(rename = "feature-type")]
  feature_type: FeatureType,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "output")]
  output: Option<HashMap<OutputType, ServerBaseDeviceFeatureOutput>>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "input")]
  input: Option<HashMap<InputType, ServerDeviceFeatureInput>>,
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[getset(get = "pub")]
  #[serde(
    rename = "feature-settings",
    skip_serializing_if = "BaseFeatureSettings::is_none",
    default
  )]
  feature_settings: BaseFeatureSettings,
}

impl ServerBaseDeviceFeature {
  pub fn as_user_device_feature(&self) -> ServerUserDeviceFeature {
    ServerUserDeviceFeature {
      id: Uuid::new_v4(),
      base_id: self.id,
      output: self.output.as_ref().and_then(|x| {
        Some(
          x.keys()
            .map(|x| (*x, ServerUserDeviceFeatureOutput::default()))
            .collect(),
        )
      }),
    }
  }
}

#[derive(
  Clone, Debug, Default, Getters, MutGetters, Setters, Serialize, Deserialize, CopyGetters,
)]
pub struct ServerUserDeviceFeature {
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[getset(get_copy = "pub")]
  #[serde(rename = "base-id")]
  base_id: Uuid,
  #[getset(get = "pub")]
  #[serde(rename = "output", skip_serializing_if = "Option::is_none")]
  output: Option<HashMap<OutputType, ServerUserDeviceFeatureOutput>>,
}

impl ServerUserDeviceFeature {
  pub fn update_output(&mut self, output_type: OutputType, output: &ServerUserDeviceFeatureOutput) {
    if let Some(ref mut output_map) = self.output {
      if output_map.contains_key(&output_type) {
        output_map.insert(output_type, output.clone());
      }
    }
  }
}

#[derive(Clone, Debug, Getters, MutGetters, Setters, Serialize, Deserialize, CopyGetters)]
pub struct ServerBaseDeviceFeatureOutput {
  #[getset(get = "pub")]
  #[serde(rename = "step-range")]
  step_range: RangeInclusive<u32>,
}

impl ServerBaseDeviceFeatureOutput {
  pub fn new(step_range: &RangeInclusive<u32>) -> Self {
    Self {
      step_range: step_range.clone(),
    }
  }
}

#[derive(
  Clone, Debug, Default, Getters, MutGetters, Setters, Serialize, Deserialize, CopyGetters,
)]
pub struct ServerUserDeviceFeatureOutput {
  #[getset(get = "pub")]
  #[serde(
    rename = "step-limit",
    default,
    skip_serializing_if = "Option::is_none",
    serialize_with = "range_serialize"
  )]
  step_limit: Option<RangeInclusive<u32>>,
  #[getset(get = "pub")]
  #[serde(
    rename = "reverse-position",
    default,
    skip_serializing_if = "Option::is_none"
  )]
  reverse_position: Option<bool>,
  #[getset(get = "pub")]
  #[serde(rename = "ignore", default, skip_serializing_if = "Option::is_none")]
  ignore: Option<bool>,
}

impl ServerUserDeviceFeatureOutput {
  pub fn new(
    step_limit: Option<RangeInclusive<u32>>,
    reverse_position: Option<bool>,
    ignore: Option<bool>,
  ) -> Self {
    Self {
      step_limit,
      reverse_position,
      ignore,
    }
  }
}

#[derive(Clone, Debug, Default, Getters, MutGetters, Setters, CopyGetters)]
pub struct ServerDeviceFeature {
  base_feature: ServerBaseDeviceFeature,
  #[getset(get_mut = "pub")]
  user_feature: ServerUserDeviceFeature,
  #[getset(get = "pub")]
  output: Option<HashMap<OutputType, ServerDeviceFeatureOutput>>,
  // input doesn't specialize across Base/User right now so we just return the base device input
}

impl PartialEq for ServerDeviceFeature {
  fn eq(&self, other: &Self) -> bool {
    self.id() == other.id()
  }
}

impl Eq for ServerDeviceFeature {
}

impl ServerDeviceFeature {
  pub fn new(
    base_feature: &ServerBaseDeviceFeature,
    user_feature: &ServerUserDeviceFeature,
  ) -> Self {
    if base_feature.id() != user_feature.base_id() {
      // TODO panic!
    }
    let output = {
      if let Some(output_map) = base_feature.output() {
        let mut output = HashMap::new();
        if let Some(user_output_map) = user_feature.output() {
          for (output_type, output_feature) in output_map {
            // TODO What if we have a key in the user map that isn't in the base map? We should remove it.
            if user_output_map.contains_key(output_type) {
              output.insert(
                *output_type,
                ServerDeviceFeatureOutput::new(
                  output_feature,
                  user_output_map.get(output_type).clone().unwrap(),
                ),
              );
            }
          }
        }
        Some(output)
      } else {
        None
      }
    };

    Self {
      output,
      base_feature: base_feature.clone(),
      user_feature: user_feature.clone(),
    }
  }

  pub fn description(&self) -> &String {
    self.base_feature.description()
  }

  pub fn feature_type(&self) -> FeatureType {
    self.base_feature.feature_type
  }

  pub fn id(&self) -> Uuid {
    self.user_feature.id()
  }

  pub fn base_id(&self) -> Uuid {
    self.base_feature.id()
  }

  pub fn alt_protocol_index(&self) -> Option<u32> {
    self.base_feature.feature_settings().alt_protocol_index()
  }

  pub fn input(&self) -> &Option<HashMap<InputType, ServerDeviceFeatureInput>> {
    self.base_feature.input()
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
      &self.base_feature.input().clone().map(|x| {
        x.iter()
          .map(|(t, a)| (*t, DeviceFeatureInput::from(a.clone())))
          .collect()
      }),
    )
  }
}

#[derive(Clone, Debug, Getters, MutGetters)]
#[getset(get = "pub")]
pub struct ServerDeviceFeatureOutput {
  base_feature: ServerBaseDeviceFeatureOutput,
  #[getset(get_mut = "pub")]
  user_feature: ServerUserDeviceFeatureOutput,
}

impl ServerDeviceFeatureOutput {
  pub fn new(
    base_feature: &ServerBaseDeviceFeatureOutput,
    user_feature: &ServerUserDeviceFeatureOutput,
  ) -> Self {
    Self {
      base_feature: base_feature.clone(),
      user_feature: user_feature.clone(),
    }
  }

  pub fn step_range(&self) -> &RangeInclusive<u32> {
    self.base_feature.step_range()
  }

  pub fn step_limit(&self) -> &RangeInclusive<u32> {
    if let Some(limit) = self.user_feature.step_limit() {
      limit
    } else {
      self.step_range()
    }
  }

  pub fn step_count(&self) -> u32 {
    if let Some(step_limit) = self.user_feature.step_limit() {
      step_limit.end() - step_limit.start()
    } else {
      self.base_feature.step_range.end() - self.base_feature.step_range.start()
    }
  }

  pub fn reverse_position(&self) -> bool {
    *self
      .user_feature
      .reverse_position()
      .as_ref()
      .unwrap_or(&false)
  }

  pub fn is_valid(&self) -> Result<(), ButtplugDeviceError> {
    let step_range = self.base_feature.step_range();
    if step_range.is_empty() {
      Err(ButtplugDeviceError::DeviceConfigurationError(
        "Step range empty.".to_string(),
      ))
    } else if let Some(step_limit) = self.user_feature.step_limit() {
      if step_limit.is_empty() {
        Err(ButtplugDeviceError::DeviceConfigurationError(
          "Step limit empty.".to_string(),
        ))
      } else if step_limit.start() < step_range.start() || step_limit.end() > step_range.end() {
        Err(ButtplugDeviceError::DeviceConfigurationError(
          "Step limit outside step range.".to_string(),
        ))
      } else {
        Ok(())
      }
    } else {
      Ok(())
    }
  }
}

impl From<ServerDeviceFeatureOutput> for DeviceFeatureOutput {
  fn from(value: ServerDeviceFeatureOutput) -> Self {
    DeviceFeatureOutput::new(value.step_count())
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
