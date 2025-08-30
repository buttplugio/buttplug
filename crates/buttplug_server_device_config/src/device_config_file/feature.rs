use std::{collections::HashSet, ops::RangeInclusive};

use buttplug_core::message::InputCommandType;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{ServerDeviceFeature, ServerDeviceFeatureOutput, ServerDeviceFeatureOutputPositionWithDurationProperties, ServerDeviceFeatureOutputValueProperties};

use super::range_sequence_serialize;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct BaseDeviceFeatureOutputValueProperties {
  value: RangeInclusive<i32>
}

impl Into<ServerDeviceFeatureOutputValueProperties> for BaseDeviceFeatureOutputValueProperties {
  fn into(self) -> ServerDeviceFeatureOutputValueProperties {
    ServerDeviceFeatureOutputValueProperties::new(&self.value.into(), false)
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct BaseDeviceFeatureOutputPositionWithDurationProperties {
  position: RangeInclusive<u32>,
  duration: RangeInclusive<u32>,
}

impl Into<ServerDeviceFeatureOutputPositionWithDurationProperties> for BaseDeviceFeatureOutputPositionWithDurationProperties {
  fn into(self) -> ServerDeviceFeatureOutputPositionWithDurationProperties {
    ServerDeviceFeatureOutputPositionWithDurationProperties::new(&self.position.into(), &self.duration.into(), false, false)
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct BaseDeviceFeatureOutput {
  #[serde(skip_serializing_if="Option::is_none")]
  vibrate: Option<BaseDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  rotate: Option<BaseDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  rotate_with_direction: Option<BaseDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  oscillate: Option<BaseDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  constrict: Option<BaseDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  heater: Option<BaseDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  led: Option<BaseDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  position: Option<BaseDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  position_with_duration: Option<BaseDeviceFeatureOutputPositionWithDurationProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  spray: Option<BaseDeviceFeatureOutputValueProperties>,
}
/*
impl Into<ServerDeviceFeatureOutput> for BaseDeviceFeatureOutput {
  fn into(self) -> ServerDeviceFeatureOutput {
    if let Some(vibrate) = self.vibrate {
    }
  }
}
*/

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserDeviceFeatureOutputValueProperties {
  #[serde(skip_serializing_if="Option::is_none")]
  value: Option<RangeInclusive<i32>>,
  #[serde(default)]
  disabled: bool,
  #[serde(default)]
  reverse: bool
}


#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserDeviceFeatureOutputPositionProperties {
  #[serde(skip_serializing_if="Option::is_none")]
  value: Option<RangeInclusive<u32>>,
  #[serde(default)]
  disabled: bool,
  #[serde(default)]
  reverse: bool
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserDeviceFeatureOutputPositionWithDurationProperties {
  #[serde(skip_serializing_if="Option::is_none")]
  position: Option<RangeInclusive<u32>>,
  #[serde(skip_serializing_if="Option::is_none")]
  duration: Option<RangeInclusive<u32>>,
  #[serde(default)]
  disabled: bool,
  #[serde(default)]
  reverse: bool
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserDeviceFeatureOutput {
  #[serde(skip_serializing_if="Option::is_none")]
  vibrate: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  rotate: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  rotate_with_direction: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  oscillate: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  constrict: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  heater: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  led: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  position: Option<UserDeviceFeatureOutputPositionProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  position_with_duration: Option<UserDeviceFeatureOutputPositionWithDurationProperties>,
  #[serde(skip_serializing_if="Option::is_none")]
  spray: Option<UserDeviceFeatureOutputValueProperties>,
}

#[derive(
  Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize,
)]
pub struct DeviceFeatureInputProperties {
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(serialize_with = "range_sequence_serialize")]
  value_range: Vec<RangeInclusive<i32>>,
  #[getset(get = "pub")]
  input_commands: HashSet<InputCommandType>
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

#[derive(
  Clone, Debug, Default, Getters, Serialize, Deserialize,
)]
#[getset(get = "pub")]
pub struct DeviceFeatureInput {
  battery: Option<DeviceFeatureInputProperties>,
  rssi: Option<DeviceFeatureInputProperties>,
  pressure: Option<DeviceFeatureInputProperties>,
  button: Option<DeviceFeatureInputProperties>
}

#[derive(
  Clone, Debug, Default, Getters, Serialize, Deserialize, CopyGetters,
)]
pub struct ConfigBaseDeviceFeature {
  #[getset(get = "pub")]
  #[serde(default)]
  description: String,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  output: Option<BaseDeviceFeatureOutput>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  input: Option<DeviceFeatureInput>,
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[getset(get = "pub")]
  #[serde(
    skip_serializing_if = "BaseFeatureSettings::is_none",
    default
  )]
  feature_settings: BaseFeatureSettings,
}

impl Into<ServerDeviceFeature> for ConfigBaseDeviceFeature {
  fn into(self) -> ServerDeviceFeature {
    ServerDeviceFeature::new(
      &self.description,
      self.id,
      None,
      self.feature_settings.alt_protocol_index,
      self.output.into(),
      self.input.into()
    ) 
  }
}

#[derive(
  Clone, Debug, Default, Getters, Serialize, Deserialize, CopyGetters,
)]
pub struct ConfigUserDeviceFeature {
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[getset(get_copy = "pub")]
  base_id: Uuid,
  #[getset(get = "pub")]
  #[serde(rename = "output", skip_serializing_if = "Option::is_none")]
  output: Option<UserDeviceFeatureOutput>
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, CopyGetters)]
pub struct BaseFeatureSettings {
  #[serde(
    rename = "alt-protocol-index",
    skip_serializing_if = "Option::is_none",
    default
  )]
  #[getset(get_copy = "pub")]
  alt_protocol_index: Option<u32>,
}

impl BaseFeatureSettings {
  pub fn is_none(&self) -> bool {
    self.alt_protocol_index.is_none()
  }
}
