// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::ButtplugDeviceConfigError;

use buttplug_core::message::{
    DeviceFeature,
    DeviceFeatureInput,
    InputCommandType, InputType, OutputType,
  };
use getset::{CopyGetters, Getters, Setters};
use serde::{
  Deserialize,
  Serialize,
  Serializer,
  ser::{self, SerializeSeq},
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

#[derive(Debug, Clone, Getters)]
#[getset(get = "pub")]
pub struct RangeWithLimit<T: PartialOrd + Clone> {
  base: RangeInclusive<T>,
  user: Option<RangeInclusive<T>>,
}

impl<T: PartialOrd + Clone> From<RangeInclusive<T>> for RangeWithLimit<T> {
  fn from(value: RangeInclusive<T>) -> Self {
    Self::new(&value)
  }
}

impl<T: PartialOrd + Clone> RangeWithLimit<T> {
  pub fn new(base: &RangeInclusive<T>) -> Self {
    Self {
      base: base.clone(),
      user: None
    }
  }

  pub fn try_new(
    base: &RangeInclusive<T>,
    user: &Option<RangeInclusive<T>>,
  ) -> Result<Self, ButtplugDeviceConfigError> {
    if let Some(user) = user {
      if user.is_empty() {
        Err(ButtplugDeviceConfigError::InvalidUserRange)
      } else {
        if *user.start() < *base.start()
          || *user.end() > *base.end()
          || *user.start() > *base.end()
          || *user.end() < *base.start()
        {
          Err(ButtplugDeviceConfigError::InvalidUserRange)
        } else {
          Ok(Self {
            base: (*base).clone(),
            user: Some((*user).clone()),
          })
        }
      }
    } else {
      if base.is_empty() {
        Err(ButtplugDeviceConfigError::BaseRangeRequired)
      } else {
        Ok(Self {
          base: (*base).clone(),
          user: None,
        })
      }
    }
  }
}

#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct ServerDeviceFeatureOutputValueProperties {
  #[getset(get = "pub")]
  value: RangeWithLimit<i32>,
  #[getset(get_copy = "pub")]
  disabled: bool,
}

impl ServerDeviceFeatureOutputValueProperties {
  pub fn new(value: &RangeWithLimit<i32>, disabled: bool) -> Self {
    Self {
      value: value.clone(),
      disabled
    }
  }
}

#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct ServerDeviceFeatureOutputPositionProperties {
  #[getset(get = "pub")]
  position: RangeWithLimit<u32>,
  #[getset(get_copy = "pub")]
  disabled: bool,
  #[getset(get_copy = "pub")]
  reverse_position: bool,
}

impl ServerDeviceFeatureOutputPositionProperties {
  pub fn new(position: &RangeWithLimit<u32>, disabled: bool, reverse_position: bool) -> Self {
    Self {
      position: position.clone(),
      disabled,
      reverse_position
    }
  }
}

#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct ServerDeviceFeatureOutputPositionWithDurationProperties {
  #[getset(get = "pub")]
  position: RangeWithLimit<u32>,
  #[getset(get = "pub")]
  duration: RangeWithLimit<u32>,
  #[getset(get_copy = "pub")]
  disabled: bool,
  #[getset(get_copy = "pub")]
  reverse_position: bool,
}

impl ServerDeviceFeatureOutputPositionWithDurationProperties {
  pub fn new(position: &RangeWithLimit<u32>, duration: &RangeWithLimit<u32>, disabled: bool, reverse_position: bool) -> Self {
    Self {
      position: position.clone(),
      duration: duration.clone(),
      disabled,
      reverse_position
    }
  }
}

#[derive(Clone, Debug, Getters, Setters, Default)]
#[getset(get = "pub", set = "pub(crate)")]
pub struct ServerDeviceFeatureOutput {
  vibrate: Option<ServerDeviceFeatureOutputValueProperties>,
  rotate: Option<ServerDeviceFeatureOutputValueProperties>,
  rotate_with_direction: Option<ServerDeviceFeatureOutputValueProperties>,
  oscillate: Option<ServerDeviceFeatureOutputValueProperties>,
  constrict: Option<ServerDeviceFeatureOutputValueProperties>,
  heater: Option<ServerDeviceFeatureOutputValueProperties>,
  led: Option<ServerDeviceFeatureOutputValueProperties>,
  position: Option<ServerDeviceFeatureOutputPositionProperties>,
  position_with_duration: Option<ServerDeviceFeatureOutputPositionWithDurationProperties>,
  spray: Option<ServerDeviceFeatureOutputValueProperties>,
}

impl ServerDeviceFeatureOutput {
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
      OutputType::Vibrate => self.vibrate.is_some()
    }
  }

  pub fn output_types(&self) -> Vec<OutputType> {
    let mut types = vec!();
    self.constrict.is_some().then(|| types.push(OutputType::Constrict));
    self.heater.is_some().then(|| types.push(OutputType::Heater));
    self.led.is_some().then(|| types.push(OutputType::Led));
    self.oscillate.is_some().then(|| types.push(OutputType::Oscillate));
    self.position.is_some().then(|| types.push(OutputType::Position));
    self.position_with_duration.is_some().then(|| types.push(OutputType::PositionWithDuration));
    self.rotate.is_some().then(|| types.push(OutputType::Rotate));
    self.rotate_with_direction.is_some().then(|| types.push(OutputType::RotateWithDirection));
    self.spray.is_some().then(|| types.push(OutputType::Spray));
    self.vibrate.is_some().then(|| types.push(OutputType::Vibrate));
    types
  }
}

#[derive(Clone, Debug, Getters)]
#[getset(get = "pub")]
pub struct ServerDeviceFeatureInputProperties {
  value_range: Vec<RangeInclusive<i32>>,
  input_commands: HashSet<InputCommandType>,
}

impl ServerDeviceFeatureInputProperties {
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

#[derive(Clone, Debug, Getters, Setters, Default)]
#[getset(get = "pub", set = "pub(crate)")]
pub struct ServerDeviceFeatureInput {
  battery: Option<ServerDeviceFeatureInputProperties>,
  rssi: Option<ServerDeviceFeatureInputProperties>,
  pressure: Option<ServerDeviceFeatureInputProperties>,
  button: Option<ServerDeviceFeatureInputProperties>,
}

impl ServerDeviceFeatureInput {
  pub fn contains(&self, input_type: InputType) -> bool {
    match input_type {
      InputType::Battery => self.battery.is_some(),
      InputType::Rssi => self.rssi.is_some(),
      InputType::Pressure => self.pressure.is_some(),
      InputType::Button => self.button.is_some(),
      InputType::Unknown => false,
    }
  }
}

#[derive(Clone, Debug, Getters, CopyGetters)]
pub struct ServerDeviceFeature {
  #[getset(get = "pub")]
  description: String,
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[getset(get_copy = "pub")]
  base_id: Option<Uuid>,
  #[getset(get_copy = "pub")]
  alt_protocol_index: Option<u32>,
  #[getset(get = "pub")]
  output: Option<ServerDeviceFeatureOutput>,
  #[getset(get = "pub")]
  input: Option<ServerDeviceFeatureInput>,
}

impl PartialEq for ServerDeviceFeature {
  fn eq(&self, other: &Self) -> bool {
    self.id() == other.id()
  }
}

impl Eq for ServerDeviceFeature {
}

impl ServerDeviceFeature {
  pub fn new(description: &str, id: Uuid, base_id: Option<Uuid>, alt_protocol_index: Option<u32>, output: &Option<ServerDeviceFeatureOutput>, input: &Option<ServerDeviceFeatureInput>) -> Self {
    Self {
      description: description.to_owned(),
      id,
      base_id,
      alt_protocol_index,
      output: output.clone(),
      input: input.clone(),
    }
  }

  /*
  pub fn as_device_feature(&self, index: u32) -> Result<DeviceFeature, ButtplugDeviceConfigError> {
    // try_collect() is still unstable so we extract the fallible-map call into a loop. This sucks.
    let mut outputs = HashMap::new();
    if let Some(output_map) = &self.output {
      for (output_type, server_output) in output_map {
        outputs.insert(
          *output_type,
          server_output.as_device_feature_output_variant(*output_type)?,
        );
      }
    }

    Ok(DeviceFeature::new(
      index,
      self.description(),
      &outputs.is_empty().then_some(outputs).or(None),
      &self.base_feature.input().clone().map(|x| {
        x.iter()
          .map(|(t, a)| (*t, DeviceFeatureInput::from(a.clone())))
          .collect()
      }),
    ))
  }
  */
}
