// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::{
  message::OutputType,
  util::{
    range::RangeInclusive,
    small_vec_enum_map::{SmallVecEnumMap, VariantKey},
  },
};

use crate::{
  ButtplugDeviceConfigError,
  RangeWithLimit,
  ServerDeviceFeature,
  ServerDeviceFeatureInput,
  ServerDeviceFeatureOutput,
  ServerDeviceFeatureOutputHwPositionWithDurationProperties,
  ServerDeviceFeatureOutputPositionProperties,
  ServerDeviceFeatureOutputValueProperties,
};
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, Default, CopyGetters)]
pub struct BaseFeatureSettings {
  #[serde(skip_serializing_if = "Option::is_none", default)]
  #[getset(get_copy = "pub")]
  alt_protocol_index: Option<u32>,
}

impl BaseFeatureSettings {
  pub fn is_none(&self) -> bool {
    self.alt_protocol_index.is_none()
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserDeviceFeatureOutputValueProperties {
  #[serde(skip_serializing_if = "Option::is_none")]
  value: Option<RangeInclusive<u32>>,
  #[serde(default)]
  disabled: bool,
}

impl UserDeviceFeatureOutputValueProperties {
  pub fn with_base_properties(
    &self,
    base: &ServerDeviceFeatureOutputValueProperties,
  ) -> Result<ServerDeviceFeatureOutputValueProperties, ButtplugDeviceConfigError> {
    Ok(ServerDeviceFeatureOutputValueProperties::new(
      RangeWithLimit::try_new(base.value.base, self.value)?,
      self.disabled,
    ))
  }
}

impl From<&ServerDeviceFeatureOutputValueProperties> for UserDeviceFeatureOutputValueProperties {
  fn from(value: &ServerDeviceFeatureOutputValueProperties) -> Self {
    Self {
      value: value.value.user,
      disabled: value.disabled,
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserDeviceFeatureOutputPositionProperties {
  #[serde(skip_serializing_if = "Option::is_none")]
  value: Option<RangeInclusive<u32>>,
  #[serde(default)]
  disabled: bool,
  #[serde(default)]
  reverse: bool,
}

impl UserDeviceFeatureOutputPositionProperties {
  pub fn with_base_properties(
    &self,
    base: &ServerDeviceFeatureOutputPositionProperties,
  ) -> Result<ServerDeviceFeatureOutputPositionProperties, ButtplugDeviceConfigError> {
    Ok(ServerDeviceFeatureOutputPositionProperties::new(
      RangeWithLimit::try_new(base.value.base, self.value)?,
      self.disabled,
      self.reverse,
    ))
  }
}

impl From<&ServerDeviceFeatureOutputPositionProperties>
  for UserDeviceFeatureOutputPositionProperties
{
  fn from(value: &ServerDeviceFeatureOutputPositionProperties) -> Self {
    Self {
      value: value.value.user,
      reverse: value.reverse_position,
      disabled: value.disabled,
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserDeviceFeatureOutputHwPositionWithDurationProperties {
  #[serde(skip_serializing_if = "Option::is_none")]
  value: Option<RangeInclusive<u32>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  duration: Option<RangeInclusive<u32>>,
  #[serde(default)]
  disabled: bool,
  #[serde(default)]
  reverse: bool,
}

impl UserDeviceFeatureOutputHwPositionWithDurationProperties {
  pub fn with_base_properties(
    &self,
    base: &ServerDeviceFeatureOutputHwPositionWithDurationProperties,
  ) -> Result<ServerDeviceFeatureOutputHwPositionWithDurationProperties, ButtplugDeviceConfigError>
  {
    Ok(
      ServerDeviceFeatureOutputHwPositionWithDurationProperties::new(
        RangeWithLimit::try_new(base.value.base, self.value)?,
        RangeWithLimit::try_new(base.duration.base, self.duration)?,
        self.disabled,
        self.reverse,
      ),
    )
  }
}

impl From<&ServerDeviceFeatureOutputHwPositionWithDurationProperties>
  for UserDeviceFeatureOutputHwPositionWithDurationProperties
{
  fn from(value: &ServerDeviceFeatureOutputHwPositionWithDurationProperties) -> Self {
    Self {
      value: value.value.user,
      duration: value.duration.user,
      reverse: value.reverse_position,
      disabled: value.disabled,
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
enum UserDeviceFeatureOutput {
  Vibrate(UserDeviceFeatureOutputValueProperties),
  Rotate(UserDeviceFeatureOutputValueProperties),
  Oscillate(UserDeviceFeatureOutputValueProperties),
  Constrict(UserDeviceFeatureOutputValueProperties),
  Temperature(UserDeviceFeatureOutputValueProperties),
  Led(UserDeviceFeatureOutputValueProperties),
  Spray(UserDeviceFeatureOutputValueProperties),
  Position(UserDeviceFeatureOutputPositionProperties),
  HwPositionWithDuration(UserDeviceFeatureOutputHwPositionWithDurationProperties),
}

impl UserDeviceFeatureOutput {
  pub fn with_base_output(
    &self,
    base: &ServerDeviceFeatureOutput,
  ) -> Result<ServerDeviceFeatureOutput, ButtplugDeviceConfigError> {
    match (self, base) {
      (Self::Vibrate(u), ServerDeviceFeatureOutput::Vibrate(b)) => Ok(
        ServerDeviceFeatureOutput::Vibrate(u.with_base_properties(b)?),
      ),
      (Self::Rotate(u), ServerDeviceFeatureOutput::Rotate(b)) => Ok(
        ServerDeviceFeatureOutput::Rotate(u.with_base_properties(b)?),
      ),
      (Self::Oscillate(u), ServerDeviceFeatureOutput::Oscillate(b)) => Ok(
        ServerDeviceFeatureOutput::Oscillate(u.with_base_properties(b)?),
      ),
      (Self::Constrict(u), ServerDeviceFeatureOutput::Constrict(b)) => Ok(
        ServerDeviceFeatureOutput::Constrict(u.with_base_properties(b)?),
      ),
      (Self::Temperature(u), ServerDeviceFeatureOutput::Temperature(b)) => Ok(
        ServerDeviceFeatureOutput::Temperature(u.with_base_properties(b)?),
      ),
      (Self::Led(u), ServerDeviceFeatureOutput::Led(b)) => {
        Ok(ServerDeviceFeatureOutput::Led(u.with_base_properties(b)?))
      }
      (Self::Spray(u), ServerDeviceFeatureOutput::Spray(b)) => {
        Ok(ServerDeviceFeatureOutput::Spray(u.with_base_properties(b)?))
      }
      (Self::Position(u), ServerDeviceFeatureOutput::Position(b)) => Ok(
        ServerDeviceFeatureOutput::Position(u.with_base_properties(b)?),
      ),
      (Self::HwPositionWithDuration(u), ServerDeviceFeatureOutput::HwPositionWithDuration(b)) => {
        Ok(ServerDeviceFeatureOutput::HwPositionWithDuration(
          u.with_base_properties(b)?,
        ))
      }
      _ => Err(ButtplugDeviceConfigError::InvalidOutputTypeConversion(
        format!(
          "user output type {:?} does not match base output type {:?}",
          self.variant_key(),
          base.output_type()
        ),
      )),
    }
  }
}

impl VariantKey for UserDeviceFeatureOutput {
  type Key = OutputType;
  fn variant_key(&self) -> OutputType {
    match self {
      Self::Vibrate(_) => OutputType::Vibrate,
      Self::Rotate(_) => OutputType::Rotate,
      Self::Oscillate(_) => OutputType::Oscillate,
      Self::Constrict(_) => OutputType::Constrict,
      Self::Temperature(_) => OutputType::Temperature,
      Self::Led(_) => OutputType::Led,
      Self::Spray(_) => OutputType::Spray,
      Self::Position(_) => OutputType::Position,
      Self::HwPositionWithDuration(_) => OutputType::HwPositionWithDuration,
    }
  }
}

impl From<&ServerDeviceFeatureOutput> for UserDeviceFeatureOutput {
  fn from(value: &ServerDeviceFeatureOutput) -> Self {
    match value {
      ServerDeviceFeatureOutput::Vibrate(p) => Self::Vibrate(p.into()),
      ServerDeviceFeatureOutput::Rotate(p) => Self::Rotate(p.into()),
      ServerDeviceFeatureOutput::Oscillate(p) => Self::Oscillate(p.into()),
      ServerDeviceFeatureOutput::Constrict(p) => Self::Constrict(p.into()),
      ServerDeviceFeatureOutput::Temperature(p) => Self::Temperature(p.into()),
      ServerDeviceFeatureOutput::Led(p) => Self::Led(p.into()),
      ServerDeviceFeatureOutput::Spray(p) => Self::Spray(p.into()),
      ServerDeviceFeatureOutput::Position(p) => Self::Position(p.into()),
      ServerDeviceFeatureOutput::HwPositionWithDuration(p) => {
        Self::HwPositionWithDuration(p.into())
      }
    }
  }
}

#[derive(Clone, Debug, Default, Getters, Serialize, Deserialize, CopyGetters)]
pub struct ConfigBaseDeviceFeature {
  #[getset(get_copy = "pub")]
  index: u32,
  #[getset(get = "pub")]
  #[serde(default)]
  description: String,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "SmallVecEnumMap::is_empty", default)]
  output: SmallVecEnumMap<ServerDeviceFeatureOutput, 1>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "SmallVecEnumMap::is_empty", default)]
  input: SmallVecEnumMap<ServerDeviceFeatureInput, 1>,
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "BaseFeatureSettings::is_none", default)]
  feature_settings: BaseFeatureSettings,
}

impl From<ConfigBaseDeviceFeature> for ServerDeviceFeature {
  fn from(val: ConfigBaseDeviceFeature) -> Self {
    ServerDeviceFeature::new(
      val.index,
      &val.description,
      val.id,
      None,
      val.feature_settings.alt_protocol_index,
      &val.output,
      &val.input,
    )
  }
}

#[derive(Clone, Debug, Default, Getters, CopyGetters, Serialize, Deserialize)]
pub struct ConfigUserDeviceFeature {
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[getset(get_copy = "pub")]
  base_id: Uuid,
  #[serde(skip_serializing_if = "SmallVecEnumMap::is_empty", default)]
  output: SmallVecEnumMap<UserDeviceFeatureOutput, 1>,
}

impl ConfigUserDeviceFeature {
  pub fn with_base_feature(
    &self,
    base_feature: &ServerDeviceFeature,
  ) -> Result<ServerDeviceFeature, ButtplugDeviceConfigError> {
    let output: SmallVecEnumMap<ServerDeviceFeatureOutput, 1> = base_feature
      .output
      .iter()
      .map(|base_out| {
        let key = base_out.variant_key();
        if let Some(user_out) = self.output.find_by_key(&key) {
          user_out.with_base_output(base_out)
        } else {
          Ok(base_out.clone())
        }
      })
      .collect::<Result<SmallVecEnumMap<ServerDeviceFeatureOutput, 1>, _>>()?;
    Ok(ServerDeviceFeature::new(
      base_feature.index(),
      &base_feature.description,
      self.id,
      Some(self.base_id),
      base_feature.alt_protocol_index,
      &output,
      &base_feature.input,
    ))
  }
}

impl TryFrom<&ServerDeviceFeature> for ConfigUserDeviceFeature {
  type Error = ButtplugDeviceConfigError;

  fn try_from(value: &ServerDeviceFeature) -> Result<Self, Self::Error> {
    Ok(Self {
      id: value.id(),
      base_id: value
        .base_id
        .ok_or(ButtplugDeviceConfigError::MissingBaseId)?,
      output: value.output.iter().map(Into::into).collect(),
    })
  }
}
