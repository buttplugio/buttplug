use std::ops::RangeInclusive;

use crate::{
  ButtplugDeviceConfigError,
  RangeWithLimit,
  ServerDeviceFeature,
  ServerDeviceFeatureInput,
  ServerDeviceFeatureOutput,
  ServerDeviceFeatureOutputPositionProperties,
  ServerDeviceFeatureOutputPositionWithDurationProperties,
  ServerDeviceFeatureOutputValueProperties,
};
use buttplug_core::util::range_serialize::option_range_serialize;
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
  #[serde(
    skip_serializing_if = "Option::is_none",
    serialize_with = "option_range_serialize"
  )]
  value: Option<RangeInclusive<u32>>,
  #[serde(default)]
  disabled: bool,
}

impl UserDeviceFeatureOutputValueProperties {
  pub fn with_base_properties(
    &self,
    base: &ServerDeviceFeatureOutputValueProperties,
  ) -> Result<ServerDeviceFeatureOutputValueProperties, ButtplugDeviceConfigError> {
    let range = RangeWithLimit::try_new(base.value().base(), &self.value)?;
    Ok(ServerDeviceFeatureOutputValueProperties::new(
      &range,
      self.disabled,
    ))
  }
}

impl From<&ServerDeviceFeatureOutputValueProperties> for UserDeviceFeatureOutputValueProperties {
  fn from(value: &ServerDeviceFeatureOutputValueProperties) -> Self {
    Self {
      value: value.value().user().clone(),
      disabled: value.disabled(),
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserDeviceFeatureOutputPositionProperties {
  #[serde(
    skip_serializing_if = "Option::is_none",
    serialize_with = "option_range_serialize"
  )]
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
    let value = RangeWithLimit::try_new(base.value().base(), &self.value)?;
    Ok(ServerDeviceFeatureOutputPositionProperties::new(
      &value,
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
      value: value.value().user().clone(),
      reverse: value.reverse_position(),
      disabled: value.disabled(),
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserDeviceFeatureOutputPositionWithDurationProperties {
  #[serde(
    skip_serializing_if = "Option::is_none",
    serialize_with = "option_range_serialize"
  )]
  value: Option<RangeInclusive<u32>>,
  #[serde(
    skip_serializing_if = "Option::is_none",
    serialize_with = "option_range_serialize"
  )]
  duration: Option<RangeInclusive<u32>>,
  #[serde(default)]
  disabled: bool,
  #[serde(default)]
  reverse: bool,
}

impl UserDeviceFeatureOutputPositionWithDurationProperties {
  pub fn with_base_properties(
    &self,
    base: &ServerDeviceFeatureOutputPositionWithDurationProperties,
  ) -> Result<ServerDeviceFeatureOutputPositionWithDurationProperties, ButtplugDeviceConfigError>
  {
    let value = RangeWithLimit::try_new(base.value().base(), &self.value)?;
    let duration = RangeWithLimit::try_new(base.duration().base(), &self.duration)?;
    Ok(
      ServerDeviceFeatureOutputPositionWithDurationProperties::new(
        &value,
        &duration,
        self.disabled,
        self.reverse,
      ),
    )
  }
}

impl From<&ServerDeviceFeatureOutputPositionWithDurationProperties>
  for UserDeviceFeatureOutputPositionWithDurationProperties
{
  fn from(value: &ServerDeviceFeatureOutputPositionWithDurationProperties) -> Self {
    Self {
      value: value.value().user().clone(),
      duration: value.duration().user().clone(),
      reverse: value.reverse_position(),
      disabled: value.disabled(),
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserDeviceFeatureOutput {
  #[serde(skip_serializing_if = "Option::is_none")]
  vibrate: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  rotate: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  oscillate: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  constrict: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  temperature: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  led: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  position: Option<UserDeviceFeatureOutputPositionProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  position_with_duration: Option<UserDeviceFeatureOutputPositionWithDurationProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  spray: Option<UserDeviceFeatureOutputValueProperties>,
}

/// Macro to apply user overrides to base output fields.
/// If base has the field, applies user override if present, otherwise keeps base value.
macro_rules! merge_output_field {
  ($output:expr, $base:expr, $user:expr, $field:ident, $setter:ident) => {
    if let Some(base_val) = $base.$field() {
      if let Some(user_val) = &$user.$field {
        $output.$setter(Some(user_val.with_base_properties(base_val)?));
      } else {
        $output.$setter(Some(base_val.clone()));
      }
    }
  };
}

impl UserDeviceFeatureOutput {
  pub fn with_base_output(
    &self,
    base_output: &ServerDeviceFeatureOutput,
  ) -> Result<ServerDeviceFeatureOutput, ButtplugDeviceConfigError> {
    let mut output = ServerDeviceFeatureOutput::default();

    merge_output_field!(output, base_output, self, vibrate, set_vibrate);
    merge_output_field!(output, base_output, self, rotate, set_rotate);
    merge_output_field!(output, base_output, self, oscillate, set_oscillate);
    merge_output_field!(output, base_output, self, constrict, set_constrict);
    merge_output_field!(output, base_output, self, temperature, set_temperature);
    merge_output_field!(output, base_output, self, led, set_led);
    merge_output_field!(output, base_output, self, spray, set_spray);
    merge_output_field!(output, base_output, self, position, set_position);
    merge_output_field!(
      output,
      base_output,
      self,
      position_with_duration,
      set_position_with_duration
    );

    Ok(output)
  }
}

impl From<&ServerDeviceFeatureOutput> for UserDeviceFeatureOutput {
  fn from(value: &ServerDeviceFeatureOutput) -> Self {
    Self {
      vibrate: value.vibrate().as_ref().map(|x| x.into()),
      rotate: value.rotate().as_ref().map(|x| x.into()),
      oscillate: value.oscillate().as_ref().map(|x| x.into()),
      constrict: value.constrict().as_ref().map(|x| x.into()),
      temperature: value.temperature().as_ref().map(|x| x.into()),
      led: value.led().as_ref().map(|x| x.into()),
      position: value.position().as_ref().map(|x| x.into()),
      position_with_duration: value.position_with_duration().as_ref().map(|x| x.into()),
      spray: value.spray().as_ref().map(|x| x.into()),
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
  #[serde(skip_serializing_if = "Option::is_none")]
  output: Option<ServerDeviceFeatureOutput>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  input: Option<ServerDeviceFeatureInput>,
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

#[derive(Clone, Debug, Default, Getters, Serialize, Deserialize, CopyGetters)]
pub struct ConfigUserDeviceFeature {
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[getset(get_copy = "pub")]
  base_id: Uuid,
  #[getset(get = "pub")]
  #[serde(rename = "output", skip_serializing_if = "Option::is_none")]
  output: Option<UserDeviceFeatureOutput>,
}

impl ConfigUserDeviceFeature {
  pub fn with_base_feature(
    &self,
    base_feature: &ServerDeviceFeature,
  ) -> Result<ServerDeviceFeature, ButtplugDeviceConfigError> {
    let output = if let Some(o) = &self.output {
      if let Some(base) = base_feature.output() {
        Some(o.with_base_output(base)?)
      } else {
        None
      }
    } else {
      None
    };
    Ok(ServerDeviceFeature::new(
      base_feature.index(),
      base_feature.description(),
      self.id,
      Some(self.base_id),
      base_feature.alt_protocol_index(),
      &output,
      base_feature.input(),
    ))
  }
}

impl From<&ServerDeviceFeature> for ConfigUserDeviceFeature {
  fn from(value: &ServerDeviceFeature) -> Self {
    Self {
      id: value.id(),
      base_id: value
        .base_id()
        .unwrap_or_else(|| panic!("Should have base id: {:?}", value)),
      output: value.output().as_ref().map(|x| x.into()),
    }
  }
}
