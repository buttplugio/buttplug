// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::ButtplugDeviceConfigError;

use buttplug_core::{
  message::{
    DeviceFeature,
    DeviceFeatureInput,
    DeviceFeatureInputProperties,
    DeviceFeatureOutput,
    DeviceFeatureOutputHwPositionWithDurationProperties,
    DeviceFeatureOutputValueProperties,
    InputCommandType,
    InputType,
    OutputType,
  },
  util::{
    range::RangeInclusive,
    small_vec_enum_map::{SmallVecEnumMap, VariantKey},
  },
};
use enumflags2::BitFlags;
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

use strum_macros::{Display, EnumDiscriminants, EnumIter, EnumString};
use uuid::Uuid;

/// Holds a combination of ranges. Base range is defined in the base device config, user range is
/// defined by the user later to be a sub-range of the base range. User range only stores in u32,
/// ranges with negatives (i.e. rotate with direction) are considered to be symettric around 0, we
/// let the system handle that conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RangeWithLimit {
  pub base: RangeInclusive<i32>,
  #[serde(skip)]
  pub user: Option<RangeInclusive<u32>>,
}

impl From<RangeInclusive<i32>> for RangeWithLimit {
  fn from(value: RangeInclusive<i32>) -> Self {
    Self::new(value)
  }
}

impl RangeWithLimit {
  pub fn new(base: RangeInclusive<i32>) -> Self {
    Self { base, user: None }
  }

  pub fn new_with_user(base: RangeInclusive<i32>, user: Option<RangeInclusive<u32>>) -> Self {
    Self { base, user }
  }

  /// Returns the effective u32 range for calculations
  pub fn internal(&self) -> RangeInclusive<u32> {
    match self.user {
      Some(user_range) => user_range,
      None => RangeInclusive::new(0, self.base.end() as u32),
    }
  }

  pub fn step_limit(&self) -> RangeInclusive<i32> {
    if self.base.start() < 0 {
      RangeInclusive::new(-(self.step_count() as i32), self.step_count() as i32)
    } else {
      RangeInclusive::new(0, self.step_count() as i32)
    }
  }

  pub fn step_count(&self) -> u32 {
    if let Some(user) = &self.user {
      user.end() - user.start()
    } else {
      self.base.end() as u32
    }
  }

  pub fn try_new(
    base: RangeInclusive<i32>,
    user: Option<RangeInclusive<u32>>,
  ) -> Result<Self, ButtplugDeviceConfigError> {
    let truncated_base = RangeInclusive::new(0, base.end() as u32);
    if let Some(user) = user {
      if user.is_empty() {
        Err(ButtplugDeviceConfigError::InvalidUserRange)
      } else if user.start() < truncated_base.start()
        || user.end() > truncated_base.end()
        || user.start() > truncated_base.end()
        || user.end() < truncated_base.start()
      {
        Err(ButtplugDeviceConfigError::InvalidUserRange)
      } else {
        Ok(Self {
          base,
          user: Some(user),
        })
      }
    } else if base.is_empty() {
      Err(ButtplugDeviceConfigError::BaseRangeRequired)
    } else {
      Ok(Self { base, user: None })
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerDeviceFeatureOutputValueProperties {
  pub value: RangeWithLimit,
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub disabled: bool,
}

impl ServerDeviceFeatureOutputValueProperties {
  pub fn new(value: RangeWithLimit, disabled: bool) -> Self {
    Self { value, disabled }
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
    let range = self.value.internal();
    let current_value = value.unsigned_abs();
    let mult = if value < 0 { -1 } else { 1 };
    if value != 0 && range.contains(range.start() + current_value) {
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
    DeviceFeatureOutputValueProperties::new(val.value.step_limit())
  }
}

#[derive(Debug, Clone, Getters, CopyGetters, Serialize, Deserialize)]
pub struct ServerDeviceFeatureOutputPositionProperties {
  pub value: RangeWithLimit,
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub disabled: bool,
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub reverse_position: bool,
}

impl ServerDeviceFeatureOutputPositionProperties {
  pub fn new(value: RangeWithLimit, disabled: bool, reverse_position: bool) -> Self {
    Self {
      value,
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
    let range = self.value.internal();
    if range.contains(range.start() + input) {
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
    DeviceFeatureOutputValueProperties::new(val.value.step_limit())
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerDeviceFeatureOutputHwPositionWithDurationProperties {
  pub value: RangeWithLimit,
  pub duration: RangeWithLimit,
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub disabled: bool,
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub reverse_position: bool,
}

impl ServerDeviceFeatureOutputHwPositionWithDurationProperties {
  pub fn new(
    value: RangeWithLimit,
    duration: RangeWithLimit,
    disabled: bool,
    reverse_position: bool,
  ) -> Self {
    Self {
      value,
      duration,
      disabled,
      reverse_position,
    }
  }

  pub fn calculate_scaled_float(&self, input: f64) -> Result<u32, ButtplugDeviceConfigError> {
    self.calculate_scaled_value((self.value.step_count() as f64 * input) as u32)
  }

  // We'll get a number from 0-x here. We'll need to calculate it with in the range we have.
  pub fn calculate_scaled_value(&self, input: u32) -> Result<u32, ButtplugDeviceConfigError> {
    let range = self.value.internal();
    if input > 0 && range.contains(range.start() + input) {
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

impl From<&ServerDeviceFeatureOutputHwPositionWithDurationProperties>
  for DeviceFeatureOutputHwPositionWithDurationProperties
{
  fn from(val: &ServerDeviceFeatureOutputHwPositionWithDurationProperties) -> Self {
    DeviceFeatureOutputHwPositionWithDurationProperties::new(
      val.value.step_limit(),
      val.duration.step_limit(),
    )
  }
}

// ServerOutputType is auto-generated as the discriminant enum of ServerDeviceFeatureOutput.
// Adding or renaming a ServerDeviceFeatureOutput variant automatically updates ServerOutputType.
#[derive(Clone, Debug, Serialize, Deserialize, EnumDiscriminants)]
#[serde(rename_all = "snake_case")]
#[strum_discriminants(name(ServerOutputType))]
#[strum_discriminants(vis(pub(crate)))]
#[strum_discriminants(derive(Display, Hash, EnumIter, EnumString, Serialize, Deserialize))]
#[strum_discriminants(serde(rename_all = "snake_case"))]
pub enum ServerDeviceFeatureOutput {
  Vibrate(ServerDeviceFeatureOutputValueProperties),
  Rotate(ServerDeviceFeatureOutputValueProperties),
  Oscillate(ServerDeviceFeatureOutputValueProperties),
  Constrict(ServerDeviceFeatureOutputValueProperties),
  Temperature(ServerDeviceFeatureOutputValueProperties),
  Led(ServerDeviceFeatureOutputValueProperties),
  Spray(ServerDeviceFeatureOutputValueProperties),
  Position(ServerDeviceFeatureOutputPositionProperties),
  HwPositionWithDuration(ServerDeviceFeatureOutputHwPositionWithDurationProperties),
}

impl ServerDeviceFeatureOutput {
  pub fn output_type(&self) -> OutputType {
    OutputType::from(ServerOutputType::from(self))
  }

  pub fn is_disabled(&self) -> bool {
    match self {
      Self::Vibrate(p)
      | Self::Rotate(p)
      | Self::Oscillate(p)
      | Self::Constrict(p)
      | Self::Temperature(p)
      | Self::Led(p)
      | Self::Spray(p) => p.disabled,
      Self::Position(p) => p.disabled,
      Self::HwPositionWithDuration(p) => p.disabled,
    }
  }

  pub fn calculate_from_value(&self, value: i32) -> Result<i32, ButtplugDeviceConfigError> {
    match self {
      Self::Vibrate(p)
      | Self::Rotate(p)
      | Self::Oscillate(p)
      | Self::Constrict(p)
      | Self::Temperature(p)
      | Self::Led(p)
      | Self::Spray(p) => p.calculate_scaled_value(value),
      Self::Position(p) => p.calculate_scaled_value(value as u32).map(|x| x as i32),
      Self::HwPositionWithDuration(p) => p.calculate_scaled_value(value as u32).map(|x| x as i32),
    }
  }

  pub fn calculate_from_float(&self, value: f64) -> Result<i32, ButtplugDeviceConfigError> {
    match self {
      Self::Vibrate(p)
      | Self::Rotate(p)
      | Self::Oscillate(p)
      | Self::Constrict(p)
      | Self::Temperature(p)
      | Self::Led(p)
      | Self::Spray(p) => p.calculate_scaled_float(value),
      Self::Position(p) => p.calculate_scaled_float(value),
      Self::HwPositionWithDuration(p) => p.calculate_scaled_float(value).map(|x| x as i32),
    }
  }

  /// Returns the value properties if this is one of the 7 simple value-type variants.
  pub fn as_value_properties(&self) -> Option<&ServerDeviceFeatureOutputValueProperties> {
    match self {
      Self::Vibrate(p)
      | Self::Rotate(p)
      | Self::Oscillate(p)
      | Self::Constrict(p)
      | Self::Temperature(p)
      | Self::Led(p)
      | Self::Spray(p) => Some(p),
      _ => None,
    }
  }
}

impl VariantKey for ServerDeviceFeatureOutput {
  type Key = OutputType;
  fn variant_key(&self) -> OutputType {
    self.output_type()
  }
}

macro_rules! impl_output_type_conversions {
  ($($variant:ident),+ $(,)?) => {
    impl From<ServerOutputType> for OutputType {
      fn from(val: ServerOutputType) -> Self {
        match val {
          $(ServerOutputType::$variant => OutputType::$variant,)+
        }
      }
    }

    impl From<OutputType> for ServerOutputType {
      fn from(val: OutputType) -> Self {
        match val {
          $(OutputType::$variant => ServerOutputType::$variant,)+
        }
      }
    }

    impl From<&ServerDeviceFeatureOutput> for DeviceFeatureOutput {
      fn from(val: &ServerDeviceFeatureOutput) -> Self {
        match val {
          $(ServerDeviceFeatureOutput::$variant(p) => DeviceFeatureOutput::$variant(p.into()),)+
        }
      }
    }
  };
}

impl_output_type_conversions![
  Vibrate,
  Rotate,
  Oscillate,
  Constrict,
  Temperature,
  Led,
  Spray,
  Position,
  HwPositionWithDuration,
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerDeviceFeatureInputProperties {
  pub value: Vec<RangeInclusive<i32>>,
  #[serde(with = "buttplug_core::util::serializers::bitflags_seq")]
  pub command: BitFlags<InputCommandType>,
}

impl ServerDeviceFeatureInputProperties {
  pub fn new(value: &[RangeInclusive<i32>], sensor_commands: &BitFlags<InputCommandType>) -> Self {
    Self {
      value: value.to_vec(),
      command: *sensor_commands,
    }
  }
}

impl From<&ServerDeviceFeatureInputProperties> for DeviceFeatureInputProperties {
  fn from(val: &ServerDeviceFeatureInputProperties) -> Self {
    DeviceFeatureInputProperties::new(&val.value, &val.command)
  }
}

// ServerInputType is auto-generated as the discriminant enum of ServerDeviceFeatureInput.
#[derive(Clone, Debug, Serialize, Deserialize, EnumDiscriminants)]
#[serde(rename_all = "snake_case")]
#[strum_discriminants(name(ServerInputType))]
#[strum_discriminants(vis(pub(crate)))]
#[strum_discriminants(derive(Display, Hash, EnumIter, EnumString, Serialize, Deserialize))]
#[strum_discriminants(serde(rename_all = "snake_case"))]
pub enum ServerDeviceFeatureInput {
  Battery(ServerDeviceFeatureInputProperties),
  Rssi(ServerDeviceFeatureInputProperties),
  Button(ServerDeviceFeatureInputProperties),
  Pressure(ServerDeviceFeatureInputProperties),
  Depth(ServerDeviceFeatureInputProperties),
  Position(ServerDeviceFeatureInputProperties),
}

impl ServerDeviceFeatureInput {
  pub fn input_type(&self) -> InputType {
    InputType::from(ServerInputType::from(self))
  }

  pub fn properties(&self) -> &ServerDeviceFeatureInputProperties {
    match self {
      Self::Battery(p)
      | Self::Rssi(p)
      | Self::Button(p)
      | Self::Pressure(p)
      | Self::Depth(p)
      | Self::Position(p) => p,
    }
  }

  pub fn can_subscribe(&self) -> bool {
    self
      .properties()
      .command
      .contains(InputCommandType::Subscribe)
  }
}

impl VariantKey for ServerDeviceFeatureInput {
  type Key = InputType;
  fn variant_key(&self) -> InputType {
    self.input_type()
  }
}

macro_rules! impl_input_type_conversions {
  ($($variant:ident),+ $(,)?) => {
    impl From<ServerInputType> for InputType {
      fn from(val: ServerInputType) -> Self {
        match val {
          $(ServerInputType::$variant => InputType::$variant,)+
        }
      }
    }

    impl From<InputType> for ServerInputType {
      fn from(val: InputType) -> Self {
        match val {
          $(InputType::$variant => ServerInputType::$variant,)+
        }
      }
    }

    impl From<&ServerDeviceFeatureInput> for DeviceFeatureInput {
      fn from(val: &ServerDeviceFeatureInput) -> Self {
        match val {
          $(ServerDeviceFeatureInput::$variant(p) => DeviceFeatureInput::$variant(p.into()),)+
        }
      }
    }
  };
}

impl_input_type_conversions![Battery, Rssi, Button, Pressure, Depth, Position,];

#[derive(Clone, Debug, Getters, CopyGetters, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerDeviceFeature {
  #[getset(get_copy = "pub")]
  #[serde(skip)]
  index: u32,
  #[serde(default)]
  pub description: String,
  #[serde(skip)]
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub base_id: Option<Uuid>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub alt_protocol_index: Option<u32>,
  #[serde(skip_serializing_if = "SmallVecEnumMap::is_empty", default)]
  pub output: SmallVecEnumMap<ServerDeviceFeatureOutput, 1>,
  #[serde(skip_serializing_if = "SmallVecEnumMap::is_empty", default)]
  pub input: SmallVecEnumMap<ServerDeviceFeatureInput, 1>,
}

impl PartialEq for ServerDeviceFeature {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
  }
}

impl Eq for ServerDeviceFeature {
}

impl Default for ServerDeviceFeature {
  fn default() -> Self {
    Self {
      index: 0,
      description: String::new(),
      id: Uuid::new_v4(),
      base_id: None,
      alt_protocol_index: None,
      output: SmallVecEnumMap::default(),
      input: SmallVecEnumMap::default(),
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
    output: &SmallVecEnumMap<ServerDeviceFeatureOutput, 1>,
    input: &SmallVecEnumMap<ServerDeviceFeatureInput, 1>,
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

  // --- Output helpers ---

  pub fn has_output(&self) -> bool {
    !self.output.is_empty()
  }

  pub fn contains_output(&self, t: OutputType) -> bool {
    self.output.contains_key(&t)
  }

  pub fn get_output(&self, t: OutputType) -> Option<&ServerDeviceFeatureOutput> {
    self.output.find_by_key(&t)
  }

  // --- Input helpers ---

  pub fn has_input(&self) -> bool {
    !self.input.is_empty()
  }

  pub fn contains_input(&self, t: InputType) -> bool {
    self.input.contains_key(&t)
  }

  pub fn get_input(&self, t: InputType) -> Option<&ServerDeviceFeatureInput> {
    self.input.find_by_key(&t)
  }

  pub fn can_subscribe(&self) -> bool {
    self.input.iter().any(|i| i.can_subscribe())
  }

  // --- Lifecycle ---

  pub fn as_new_user_feature(&self) -> Self {
    let mut new_feature = self.clone();
    new_feature.base_id = Some(self.id);
    new_feature.id = Uuid::new_v4();
    new_feature
  }

  pub fn as_device_feature(&self) -> Result<DeviceFeature, ButtplugDeviceConfigError> {
    let output: SmallVecEnumMap<DeviceFeatureOutput, 1> = self
      .output
      .iter()
      .filter(|o| !o.is_disabled())
      .map(|o| o.into())
      .collect();
    let input: SmallVecEnumMap<DeviceFeatureInput, 1> =
      self.input.iter().map(|i| i.into()).collect();
    Ok(DeviceFeature::new(
      self.index,
      &self.description,
      &output,
      &input,
    ))
  }
}
