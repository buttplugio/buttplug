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
use std::{collections::HashSet, ops::RangeInclusive};
use uuid::Uuid;

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

#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct ServerDeviceFeatureOutputValueProperties {
  #[getset(get = "pub")]
  value: RangeWithLimit,
  #[getset(get_copy = "pub")]
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
    if value > 0 && range.contains(&(range.start() + current_value)) {
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

#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct ServerDeviceFeatureOutputPositionProperties {
  #[getset(get = "pub")]
  position: RangeWithLimit,
  #[getset(get_copy = "pub")]
  disabled: bool,
  #[getset(get_copy = "pub")]
  reverse_position: bool,
}

impl ServerDeviceFeatureOutputPositionProperties {
  pub fn new(position: &RangeWithLimit, disabled: bool, reverse_position: bool) -> Self {
    Self {
      position: position.clone(),
      disabled,
      reverse_position,
    }
  }

  pub fn calculate_scaled_float(&self, value: f64) -> Result<i32, ButtplugDeviceConfigError> {
    if !(0.0..=1.0).contains(&value) {
      Err(ButtplugDeviceConfigError::InvalidFloatConversion(value))
    } else {
      self
        .calculate_scaled_value((self.position.step_count() as f64 * value).ceil() as u32)
        .map(|x| x as i32)
    }
  }

  // We'll get a number from 0-x here. We'll need to calculate it with in the range we have.
  pub fn calculate_scaled_value(&self, value: u32) -> Result<u32, ButtplugDeviceConfigError> {
    let range = if let Some(user_range) = self.position.user() {
      user_range
    } else {
      self.position.internal_base()
    };
    if range.contains(&(range.start() + value)) {
      if self.reverse_position {
        Ok(range.end() - value)
      } else {
        Ok(range.start() + value)
      }
    } else {
      Err(ButtplugDeviceConfigError::InvalidOutputValue(
        value as i32,
        format!("{:?}", range),
      ))
    }
  }
}

impl From<&ServerDeviceFeatureOutputPositionProperties> for DeviceFeatureOutputValueProperties {
  fn from(val: &ServerDeviceFeatureOutputPositionProperties) -> Self {
    DeviceFeatureOutputValueProperties::new(&val.position().step_limit())
  }
}

#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct ServerDeviceFeatureOutputPositionWithDurationProperties {
  #[getset(get = "pub")]
  position: RangeWithLimit,
  #[getset(get = "pub")]
  duration: RangeWithLimit,
  #[getset(get_copy = "pub")]
  disabled: bool,
  #[getset(get_copy = "pub")]
  reverse_position: bool,
}

impl ServerDeviceFeatureOutputPositionWithDurationProperties {
  pub fn new(
    position: &RangeWithLimit,
    duration: &RangeWithLimit,
    disabled: bool,
    reverse_position: bool,
  ) -> Self {
    Self {
      position: position.clone(),
      duration: duration.clone(),
      disabled,
      reverse_position,
    }
  }

  pub fn calculate_scaled_float(&self, value: f64) -> Result<u32, ButtplugDeviceConfigError> {
    self.calculate_scaled_value((self.position.step_count() as f64 * value) as u32)
  }

  // We'll get a number from 0-x here. We'll need to calculate it with in the range we have.
  pub fn calculate_scaled_value(&self, value: u32) -> Result<u32, ButtplugDeviceConfigError> {
    let range = if let Some(user_range) = self.position.user() {
      user_range
    } else {
      self.position.internal_base()
    };
    if value > 0 && range.contains(&(range.start() + value)) {
      if self.reverse_position {
        Ok(range.end() - value)
      } else {
        Ok(range.start() + value)
      }
    } else if value == 0 {
      Ok(0)
    } else {
      Err(ButtplugDeviceConfigError::InvalidOutputValue(
        value as i32,
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
      &val.position().step_limit(),
      &val.duration().step_limit(),
    )
  }
}

#[derive(Clone, Debug, Getters, Setters, Default)]
#[getset(get = "pub", set = "pub")]
pub struct ServerDeviceFeatureOutput {
  vibrate: Option<ServerDeviceFeatureOutputValueProperties>,
  rotate: Option<ServerDeviceFeatureOutputValueProperties>,
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
      OutputType::Spray => self.spray.is_some(),
      OutputType::Unknown => false,
      OutputType::Vibrate => self.vibrate.is_some(),
    }
  }

  pub fn output_types(&self) -> Vec<OutputType> {
    let mut types = vec![];
    self
      .constrict
      .is_some()
      .then(|| types.push(OutputType::Constrict));
    self
      .heater
      .is_some()
      .then(|| types.push(OutputType::Heater));
    self.led.is_some().then(|| types.push(OutputType::Led));
    self
      .oscillate
      .is_some()
      .then(|| types.push(OutputType::Oscillate));
    self
      .position
      .is_some()
      .then(|| types.push(OutputType::Position));
    self
      .position_with_duration
      .is_some()
      .then(|| types.push(OutputType::PositionWithDuration));
    self
      .rotate
      .is_some()
      .then(|| types.push(OutputType::Rotate));
    self.spray.is_some().then(|| types.push(OutputType::Spray));
    self
      .vibrate
      .is_some()
      .then(|| types.push(OutputType::Vibrate));
    types
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
      OutputType::Heater => self.heater.as_ref().map_or(
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
      OutputType::Heater => self.heater.as_ref().map_or(
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
    val.heater.as_ref().map(|x| builder.heater(x.into()));
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

impl From<&ServerDeviceFeatureInputProperties> for DeviceFeatureInputProperties {
  fn from(val: &ServerDeviceFeatureInputProperties) -> Self {
    DeviceFeatureInputProperties::new(&val.value_range, &val.input_commands)
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

impl From<ServerDeviceFeatureInput> for DeviceFeatureInput {
  fn from(val: ServerDeviceFeatureInput) -> Self {
    let mut builder = DeviceFeatureInputBuilder::default();
    val.battery.as_ref().map(|x| builder.battery(x.into()));
    val.rssi.as_ref().map(|x| builder.rssi(x.into()));
    val.pressure.as_ref().map(|x| builder.pressure(x.into()));
    val.button.as_ref().map(|x| builder.button(x.into()));
    builder.build().expect("Infallible")
  }
}

#[derive(Clone, Debug, Getters, CopyGetters, Setters)]
pub struct ServerDeviceFeature {
  #[getset(get = "pub")]
  description: String,
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[getset(get_copy = "pub")]
  base_id: Option<Uuid>,
  #[getset(get_copy = "pub")]
  alt_protocol_index: Option<u32>,
  #[getset(get = "pub", set = "pub")]
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
  pub fn new(
    description: &str,
    id: Uuid,
    base_id: Option<Uuid>,
    alt_protocol_index: Option<u32>,
    output: &Option<ServerDeviceFeatureOutput>,
    input: &Option<ServerDeviceFeatureInput>,
  ) -> Self {
    Self {
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

  pub fn as_device_feature(&self, index: u32) -> Result<DeviceFeature, ButtplugDeviceConfigError> {
    Ok(DeviceFeature::new(
      index,
      self.description(),
      &self.output.as_ref().map(|x| x.clone().into()),
      &self.input.as_ref().map(|x| x.clone().into()),
    ))
  }
}
