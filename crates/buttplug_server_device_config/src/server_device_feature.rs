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
  DeviceFeatureInputBuilder,
  DeviceFeatureInputProperties,
  DeviceFeatureOutput,
  DeviceFeatureOutputBuilder,
  DeviceFeatureOutputPositionWithDurationProperties,
  DeviceFeatureOutputValueProperties,
  InputCommandType,
  InputType,
  OutputType,
};
use getset::{CopyGetters, Getters, Setters};
use serde::{
  de::{self, Deserializer, SeqAccess, Visitor},
  ser::SerializeSeq,
  Deserialize, Serialize, Serializer,
};
use std::{collections::HashSet, fmt, ops::RangeInclusive};
use uuid::Uuid;

/// Serde helper module for serializing/deserializing Vec<RangeInclusive<i32>> as [[start, end], ...]
mod range_vec_serde {
  use super::*;

  pub fn serialize<S>(ranges: &Vec<RangeInclusive<i32>>, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let arrays: Vec<[i32; 2]> = ranges.iter().map(|r| [*r.start(), *r.end()]).collect();
    arrays.serialize(serializer)
  }

  pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<RangeInclusive<i32>>, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct RangeVecVisitor;

    impl<'de> Visitor<'de> for RangeVecVisitor {
      type Value = Vec<RangeInclusive<i32>>;

      fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an array of two-element arrays [[start, end], ...]")
      }

      fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
      where
        A: SeqAccess<'de>,
      {
        let mut ranges = Vec::new();
        while let Some([start, end]) = seq.next_element::<[i32; 2]>()? {
          ranges.push(start..=end);
        }
        Ok(ranges)
      }
    }

    deserializer.deserialize_seq(RangeVecVisitor)
  }
}

/// Holds a combination of ranges. Base range is defined in the base device config, user range is
/// defined by the user later to be a sub-range of the base range. User range only stores in u32,
/// ranges with negatives (i.e. rotate with direction) are considered to be symettric around 0, we
/// let the system handle that conversion.
#[derive(Debug, Clone, Getters)]
#[getset(get = "pub")]
pub struct RangeWithLimit {
  base: RangeInclusive<i32>,
  internal_base: RangeInclusive<u32>,
  user: Option<RangeInclusive<u32>>,
}

impl From<RangeInclusive<i32>> for RangeWithLimit {
  fn from(value: RangeInclusive<i32>) -> Self {
    Self::new(&value)
  }
}

impl RangeWithLimit {
  pub fn new(base: &RangeInclusive<i32>) -> Self {
    Self {
      base: base.clone(),
      internal_base: RangeInclusive::new(0, *base.end() as u32),
      user: None,
    }
  }

  pub fn new_with_user(base: &RangeInclusive<i32>, user: &Option<RangeInclusive<u32>>) -> Self {
    Self {
      base: base.clone(),
      internal_base: RangeInclusive::new(0, *base.end() as u32),
      user: user.clone(),
    }
  }

  pub fn step_limit(&self) -> RangeInclusive<i32> {
    if *self.base.start() < 0 {
      RangeInclusive::new(-(self.step_count() as i32), self.step_count() as i32)
    } else {
      RangeInclusive::new(0, self.step_count() as i32)
    }
  }

  pub fn step_count(&self) -> u32 {
    if let Some(user) = &self.user {
      *user.end() - *user.start()
    } else {
      *self.base.end() as u32
    }
  }

  pub fn try_new(
    base: &RangeInclusive<i32>,
    user: &Option<RangeInclusive<u32>>,
  ) -> Result<Self, ButtplugDeviceConfigError> {
    let truncated_base = RangeInclusive::new(0, *base.end() as u32);
    if let Some(user) = user {
      if user.is_empty() {
        Err(ButtplugDeviceConfigError::InvalidUserRange)
      } else if *user.start() < *truncated_base.start()
        || *user.end() > *truncated_base.end()
        || *user.start() > *truncated_base.end()
        || *user.end() < *truncated_base.start()
      {
        Err(ButtplugDeviceConfigError::InvalidUserRange)
      } else {
        Ok(Self {
          base: (*base).clone(),
          internal_base: truncated_base,
          user: Some((*user).clone()),
        })
      }
    } else if base.is_empty() {
      Err(ButtplugDeviceConfigError::BaseRangeRequired)
    } else {
      Ok(Self {
        base: (*base).clone(),
        internal_base: truncated_base,
        user: None,
      })
    }
  }
}

impl Serialize for RangeWithLimit {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let mut seq = serializer.serialize_seq(Some(2))?;
    seq.serialize_element(self.base.start())?;
    seq.serialize_element(self.base.end())?;
    seq.end()
  }
}

impl<'de> Deserialize<'de> for RangeWithLimit {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct RangeVisitor;

    impl<'de> Visitor<'de> for RangeVisitor {
      type Value = RangeWithLimit;

      fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a two-element array [start, end]")
      }

      fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
      where
        A: SeqAccess<'de>,
      {
        let start: i32 = seq
          .next_element()?
          .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let end: i32 = seq
          .next_element()?
          .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        Ok(RangeWithLimit::new(&(start..=end)))
      }
    }

    deserializer.deserialize_seq(RangeVisitor)
  }
}

#[derive(Debug, Clone, Getters, CopyGetters, Serialize, Deserialize)]
pub struct ServerDeviceFeatureOutputValueProperties {
  #[getset(get = "pub")]
  value: RangeWithLimit,
  #[getset(get_copy = "pub")]
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  disabled: bool,
}

impl ServerDeviceFeatureOutputValueProperties {
  pub fn new(value: &RangeWithLimit, disabled: bool) -> Self {
    Self {
      value: value.clone(),
      disabled,
    }
  }

  pub fn calculate_scaled_float(&self, value: f64) -> Result<i32, ButtplugDeviceConfigError> {
    if !(0.0..=1.0).contains(&value) {
      Err(ButtplugDeviceConfigError::InvalidFloatConversion(value))
    } else {
      let value = if value < 0.000001 { 0f64 } else { value };
      self.calculate_scaled_value((self.value.step_count() as f64 * value).ceil() as i32)
    }
  }

  // We'll get a number from 0-x here. We'll need to calculate it with in the range we have. We'll
  // consider negative ranges symmetric.
  pub fn calculate_scaled_value(&self, value: i32) -> Result<i32, ButtplugDeviceConfigError> {
    let range = if let Some(user_range) = self.value.user() {
      user_range
    } else {
      self.value.internal_base()
    };
    let current_value = value.unsigned_abs();
    let mult = if value < 0 { -1 } else { 1 };
    if value != 0 && range.contains(&(range.start() + current_value)) {
      Ok((range.start() + current_value) as i32 * mult)
    } else if value == 0 {
      Ok(0)
    } else {
      Err(ButtplugDeviceConfigError::InvalidOutputValue(
        value,
        format!("{:?}", range),
      ))
    }
  }
}

impl From<&ServerDeviceFeatureOutputValueProperties> for DeviceFeatureOutputValueProperties {
  fn from(val: &ServerDeviceFeatureOutputValueProperties) -> Self {
    DeviceFeatureOutputValueProperties::new(&val.value().step_limit())
  }
}

#[derive(Debug, Clone, Getters, CopyGetters, Serialize, Deserialize)]
pub struct ServerDeviceFeatureOutputPositionProperties {
  #[getset(get = "pub")]
  value: RangeWithLimit,
  #[getset(get_copy = "pub")]
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  disabled: bool,
  #[getset(get_copy = "pub")]
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  reverse_position: bool,
}

impl ServerDeviceFeatureOutputPositionProperties {
  pub fn new(value: &RangeWithLimit, disabled: bool, reverse_position: bool) -> Self {
    Self {
      value: value.clone(),
      disabled,
      reverse_position,
    }
  }

  pub fn calculate_scaled_float(&self, input: f64) -> Result<i32, ButtplugDeviceConfigError> {
    if !(0.0..=1.0).contains(&input) {
      Err(ButtplugDeviceConfigError::InvalidFloatConversion(input))
    } else {
      self
        .calculate_scaled_value((self.value.step_count() as f64 * input).ceil() as u32)
        .map(|x| x as i32)
    }
  }

  // We'll get a number from 0-x here. We'll need to calculate it with in the range we have.
  pub fn calculate_scaled_value(&self, input: u32) -> Result<u32, ButtplugDeviceConfigError> {
    let range = if let Some(user_range) = self.value.user() {
      user_range
    } else {
      self.value.internal_base()
    };
    if range.contains(&(range.start() + input)) {
      if self.reverse_position {
        Ok(range.end() - input)
      } else {
        Ok(range.start() + input)
      }
    } else {
      Err(ButtplugDeviceConfigError::InvalidOutputValue(
        input as i32,
        format!("{:?}", range),
      ))
    }
  }
}

impl From<&ServerDeviceFeatureOutputPositionProperties> for DeviceFeatureOutputValueProperties {
  fn from(val: &ServerDeviceFeatureOutputPositionProperties) -> Self {
    DeviceFeatureOutputValueProperties::new(&val.value().step_limit())
  }
}

#[derive(Debug, Clone, Getters, CopyGetters, Serialize, Deserialize)]
pub struct ServerDeviceFeatureOutputPositionWithDurationProperties {
  #[getset(get = "pub")]
  value: RangeWithLimit,
  #[getset(get = "pub")]
  duration: RangeWithLimit,
  #[getset(get_copy = "pub")]
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  disabled: bool,
  #[getset(get_copy = "pub")]
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  reverse_position: bool,
}

impl ServerDeviceFeatureOutputPositionWithDurationProperties {
  pub fn new(
    value: &RangeWithLimit,
    duration: &RangeWithLimit,
    disabled: bool,
    reverse_position: bool,
  ) -> Self {
    Self {
      value: value.clone(),
      duration: duration.clone(),
      disabled,
      reverse_position,
    }
  }

  pub fn calculate_scaled_float(&self, input: f64) -> Result<u32, ButtplugDeviceConfigError> {
    self.calculate_scaled_value((self.value.step_count() as f64 * input) as u32)
  }

  // We'll get a number from 0-x here. We'll need to calculate it with in the range we have.
  pub fn calculate_scaled_value(&self, input: u32) -> Result<u32, ButtplugDeviceConfigError> {
    let range = if let Some(user_range) = self.value.user() {
      user_range
    } else {
      self.value.internal_base()
    };
    if input > 0 && range.contains(&(range.start() + input)) {
      if self.reverse_position {
        Ok(range.end() - input)
      } else {
        Ok(range.start() + input)
      }
    } else if input == 0 {
      Ok(0)
    } else {
      Err(ButtplugDeviceConfigError::InvalidOutputValue(
        input as i32,
        format!("{:?}", range),
      ))
    }
  }
}

impl From<&ServerDeviceFeatureOutputPositionWithDurationProperties>
  for DeviceFeatureOutputPositionWithDurationProperties
{
  fn from(val: &ServerDeviceFeatureOutputPositionWithDurationProperties) -> Self {
    DeviceFeatureOutputPositionWithDurationProperties::new(
      &val.value().step_limit(),
      &val.duration().step_limit(),
    )
  }
}

#[derive(Clone, Debug, Getters, Setters, Default, Serialize, Deserialize)]
#[serde(default)]
#[getset(get = "pub", set = "pub")]
pub struct ServerDeviceFeatureOutput {
  #[serde(skip_serializing_if = "Option::is_none")]
  vibrate: Option<ServerDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  rotate: Option<ServerDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  oscillate: Option<ServerDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  constrict: Option<ServerDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  temperature: Option<ServerDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  led: Option<ServerDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  position: Option<ServerDeviceFeatureOutputPositionProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  position_with_duration: Option<ServerDeviceFeatureOutputPositionWithDurationProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  spray: Option<ServerDeviceFeatureOutputValueProperties>,
}

impl ServerDeviceFeatureOutput {
  pub fn contains(&self, output_type: OutputType) -> bool {
    match output_type {
      OutputType::Constrict => self.constrict.is_some(),
      OutputType::Temperature => self.temperature.is_some(),
      OutputType::Led => self.led.is_some(),
      OutputType::Oscillate => self.oscillate.is_some(),
      OutputType::Position => self.position.is_some(),
      OutputType::PositionWithDuration => self.position_with_duration.is_some(),
      OutputType::Rotate => self.rotate.is_some(),
      OutputType::Spray => self.spray.is_some(),
      OutputType::Unknown => false,
      OutputType::Vibrate => self.vibrate.is_some(),
    }
  }

  pub fn output_types(&self) -> Vec<OutputType> {
    [
      (self.vibrate.is_some(), OutputType::Vibrate),
      (self.rotate.is_some(), OutputType::Rotate),
      (self.oscillate.is_some(), OutputType::Oscillate),
      (self.constrict.is_some(), OutputType::Constrict),
      (self.temperature.is_some(), OutputType::Temperature),
      (self.led.is_some(), OutputType::Led),
      (self.position.is_some(), OutputType::Position),
      (self.position_with_duration.is_some(), OutputType::PositionWithDuration),
      (self.spray.is_some(), OutputType::Spray),
    ]
    .into_iter()
    .filter_map(|(present, ot)| present.then_some(ot))
    .collect()
  }

  pub fn calculate_from_value(
    &self,
    output_type: OutputType,
    value: i32,
  ) -> Result<i32, ButtplugDeviceConfigError> {
    // TODO just fucking do some trait implementations for calculation methods and clean this up for fuck sake. :c
    match output_type {
      OutputType::Constrict => self.constrict.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_value(value),
      ),
      OutputType::Temperature => self.temperature.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_value(value),
      ),
      OutputType::Led => self.led.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_value(value),
      ),
      OutputType::Oscillate => self.oscillate.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_value(value),
      ),
      OutputType::Position => self.position.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_value(value as u32).map(|x| x as i32),
      ),
      OutputType::PositionWithDuration => self.position_with_duration.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_value(value as u32).map(|x| x as i32),
      ),
      OutputType::Rotate => self.rotate.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_value(value),
      ),
      OutputType::Spray => self.spray.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_value(value),
      ),
      OutputType::Unknown => Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
      OutputType::Vibrate => self.vibrate.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_value(value),
      ),
    }
  }

  pub fn calculate_from_float(
    &self,
    output_type: OutputType,
    value: f64,
  ) -> Result<i32, ButtplugDeviceConfigError> {
    match output_type {
      OutputType::Constrict => self.constrict.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_float(value),
      ),
      OutputType::Temperature => self.temperature.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_float(value),
      ),
      OutputType::Led => self.led.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_float(value),
      ),
      OutputType::Oscillate => self.oscillate.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_float(value),
      ),
      OutputType::Position => self.position.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_float(value),
      ),
      OutputType::PositionWithDuration => self.position_with_duration.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_float(value).map(|x| x as i32),
      ),
      OutputType::Rotate => self.rotate.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_float(value),
      ),
      OutputType::Spray => self.spray.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_float(value),
      ),
      OutputType::Unknown => Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
      OutputType::Vibrate => self.vibrate.as_ref().map_or(
        Err(ButtplugDeviceConfigError::InvalidOutput(output_type)),
        |x| x.calculate_scaled_float(value),
      ),
    }
  }
}

impl From<ServerDeviceFeatureOutput> for DeviceFeatureOutput {
  fn from(val: ServerDeviceFeatureOutput) -> Self {
    let mut builder = DeviceFeatureOutputBuilder::default();
    val.vibrate.as_ref().map(|x| builder.vibrate(x.into()));
    val.rotate.as_ref().map(|x| builder.rotate(x.into()));
    val.oscillate.as_ref().map(|x| builder.oscillate(x.into()));
    val.constrict.as_ref().map(|x| builder.constrict(x.into()));
    val
      .temperature
      .as_ref()
      .map(|x| builder.temperature(x.into()));
    val.led.as_ref().map(|x| builder.led(x.into()));
    val.position.as_ref().map(|x| builder.position(x.into()));
    val
      .position_with_duration
      .as_ref()
      .map(|x| builder.position_with_duration(x.into()));
    val.spray.as_ref().map(|x| builder.spray(x.into()));
    builder.build().expect("Infallible")
  }
}

#[derive(Clone, Debug, Getters, Serialize, Deserialize)]
#[getset(get = "pub")]
pub struct ServerDeviceFeatureInputProperties {
  #[serde(with = "range_vec_serde")]
  value: Vec<RangeInclusive<i32>>,
  command: HashSet<InputCommandType>,
}

impl ServerDeviceFeatureInputProperties {
  pub fn new(
    value: &Vec<RangeInclusive<i32>>,
    sensor_commands: &HashSet<InputCommandType>,
  ) -> Self {
    Self {
      value: value.clone(),
      command: sensor_commands.clone(),
    }
  }
}

impl From<&ServerDeviceFeatureInputProperties> for DeviceFeatureInputProperties {
  fn from(val: &ServerDeviceFeatureInputProperties) -> Self {
    DeviceFeatureInputProperties::new(&val.value, &val.command)
  }
}

#[derive(Clone, Debug, Getters, Setters, Default, Serialize, Deserialize)]
#[serde(default)]
#[getset(get = "pub", set = "pub(crate)")]
pub struct ServerDeviceFeatureInput {
  #[serde(skip_serializing_if = "Option::is_none")]
  battery: Option<ServerDeviceFeatureInputProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  rssi: Option<ServerDeviceFeatureInputProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pressure: Option<ServerDeviceFeatureInputProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  button: Option<ServerDeviceFeatureInputProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  depth: Option<ServerDeviceFeatureInputProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  position: Option<ServerDeviceFeatureInputProperties>,
}

impl ServerDeviceFeatureInput {
  pub fn contains(&self, input_type: InputType) -> bool {
    match input_type {
      InputType::Battery => self.battery.is_some(),
      InputType::Rssi => self.rssi.is_some(),
      InputType::Pressure => self.pressure.is_some(),
      InputType::Button => self.button.is_some(),
      InputType::Depth => self.depth.is_some(),
      InputType::Position => self.position.is_some(),
      InputType::Unknown => false,
    }
  }

  pub fn can_subscribe(&self) -> bool {
    [
      &self.battery,
      &self.rssi,
      &self.pressure,
      &self.button,
      &self.depth,
      &self.position,
    ]
    .iter()
    .any(|input| {
      input
        .as_ref()
        .map_or(false, |i| i.command.contains(&InputCommandType::Subscribe))
    })
  }
}

impl From<ServerDeviceFeatureInput> for DeviceFeatureInput {
  fn from(val: ServerDeviceFeatureInput) -> Self {
    let mut builder = DeviceFeatureInputBuilder::default();
    val.battery.as_ref().map(|x| builder.battery(x.into()));
    val.rssi.as_ref().map(|x| builder.rssi(x.into()));
    val.pressure.as_ref().map(|x| builder.pressure(x.into()));
    val.button.as_ref().map(|x| builder.button(x.into()));
    val.depth.as_ref().map(|x| builder.depth(x.into()));
    val.position.as_ref().map(|x| builder.position(x.into()));
    builder.build().expect("Infallible")
  }
}

#[derive(Clone, Debug, Getters, CopyGetters, Setters, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerDeviceFeature {
  #[getset(get_copy = "pub")]
  #[serde(skip)]
  index: u32,
  #[getset(get = "pub")]
  #[serde(default)]
  description: String,
  #[getset(get_copy = "pub")]
  #[serde(skip)]
  id: Uuid,
  #[getset(get_copy = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  base_id: Option<Uuid>,
  #[getset(get_copy = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  alt_protocol_index: Option<u32>,
  #[getset(get = "pub", set = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  output: Option<ServerDeviceFeatureOutput>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  input: Option<ServerDeviceFeatureInput>,
}

impl PartialEq for ServerDeviceFeature {
  fn eq(&self, other: &Self) -> bool {
    self.id() == other.id()
  }
}

impl Eq for ServerDeviceFeature {}

impl Default for ServerDeviceFeature {
  fn default() -> Self {
    Self {
      index: 0,
      description: String::new(),
      id: Uuid::new_v4(),
      base_id: None,
      alt_protocol_index: None,
      output: None,
      input: None,
    }
  }
}

impl ServerDeviceFeature {
  pub fn new(
    index: u32,
    description: &str,
    id: Uuid,
    base_id: Option<Uuid>,
    alt_protocol_index: Option<u32>,
    output: &Option<ServerDeviceFeatureOutput>,
    input: &Option<ServerDeviceFeatureInput>,
  ) -> Self {
    Self {
      index,
      description: description.to_owned(),
      id,
      base_id,
      alt_protocol_index,
      output: output.clone(),
      input: input.clone(),
    }
  }

  pub fn as_new_user_feature(&self) -> Self {
    let mut new_feature = self.clone();
    new_feature.base_id = Some(self.id);
    new_feature.id = Uuid::new_v4();
    new_feature
  }

  pub fn as_device_feature(&self) -> Result<DeviceFeature, ButtplugDeviceConfigError> {
    Ok(DeviceFeature::new(
      self.index,
      self.description(),
      &self.output.as_ref().map(|x| x.clone().into()),
      &self.input.as_ref().map(|x| x.clone().into()),
    ))
  }
}
