use std::{collections::HashSet, ops::RangeInclusive};

use crate::{
  ButtplugDeviceConfigError,
  RangeWithLimit,
  ServerDeviceFeature,
  ServerDeviceFeatureInput,
  ServerDeviceFeatureInputProperties,
  ServerDeviceFeatureOutput,
  ServerDeviceFeatureOutputPositionProperties,
  ServerDeviceFeatureOutputPositionWithDurationProperties,
  ServerDeviceFeatureOutputValueProperties,
};
use buttplug_core::{message::InputCommandType, util::range_serialize::range_sequence_serialize};
use getset::{CopyGetters, Getters, MutGetters, Setters};
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
struct BaseDeviceFeatureOutputValueProperties {
  value: RangeInclusive<i32>,
}

impl Into<ServerDeviceFeatureOutputValueProperties> for BaseDeviceFeatureOutputValueProperties {
  fn into(self) -> ServerDeviceFeatureOutputValueProperties {
    ServerDeviceFeatureOutputValueProperties::new(&self.value.into(), false)
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct BaseDeviceFeatureOutputPositionProperties {
  value: RangeInclusive<i32>,
}

impl Into<ServerDeviceFeatureOutputPositionProperties>
  for BaseDeviceFeatureOutputPositionProperties
{
  fn into(self) -> ServerDeviceFeatureOutputPositionProperties {
    ServerDeviceFeatureOutputPositionProperties::new(&self.value.into(), false, false)
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct BaseDeviceFeatureOutputPositionWithDurationProperties {
  position: RangeInclusive<i32>,
  duration: RangeInclusive<i32>,
}

impl Into<ServerDeviceFeatureOutputPositionWithDurationProperties>
  for BaseDeviceFeatureOutputPositionWithDurationProperties
{
  fn into(self) -> ServerDeviceFeatureOutputPositionWithDurationProperties {
    ServerDeviceFeatureOutputPositionWithDurationProperties::new(
      &self.position.into(),
      &self.duration.into(),
      false,
      false,
    )
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct BaseDeviceFeatureOutput {
  #[serde(skip_serializing_if = "Option::is_none")]
  vibrate: Option<BaseDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  rotate: Option<BaseDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  rotate_with_direction: Option<BaseDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  oscillate: Option<BaseDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  constrict: Option<BaseDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  heater: Option<BaseDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  led: Option<BaseDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  position: Option<BaseDeviceFeatureOutputPositionProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  position_with_duration: Option<BaseDeviceFeatureOutputPositionWithDurationProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  spray: Option<BaseDeviceFeatureOutputValueProperties>,
}

impl Into<ServerDeviceFeatureOutput> for BaseDeviceFeatureOutput {
  fn into(self) -> ServerDeviceFeatureOutput {
    let mut output = ServerDeviceFeatureOutput::default();
    if let Some(vibrate) = self.vibrate {
      output.set_vibrate(Some(vibrate.into()));
    }
    if let Some(rotate) = self.rotate {
      output.set_rotate(Some(rotate.into()));
    }
    if let Some(rotate_with_direction) = self.rotate_with_direction {
      output.set_rotate_with_direction(Some(rotate_with_direction.into()));
    }
    if let Some(oscillate) = self.oscillate {
      output.set_oscillate(Some(oscillate.into()));
    }
    if let Some(constrict) = self.constrict {
      output.set_constrict(Some(constrict.into()));
    }
    if let Some(heater) = self.heater {
      output.set_heater(Some(heater.into()));
    }
    if let Some(led) = self.led {
      output.set_led(Some(led.into()));
    }
    if let Some(position) = self.position {
      output.set_position(Some(position.into()));
    }
    if let Some(position_with_duration) = self.position_with_duration {
      output.set_position_with_duration(Some(position_with_duration.into()));
    }
    if let Some(spray) = self.spray {
      output.set_spray(Some(spray.into()));
    }
    output
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
    let value = RangeWithLimit::try_new(base.position().base(), &self.value)?;
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
      value: value.position().user().clone(),
      reverse: value.reverse_position(),
      disabled: value.disabled(),
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct UserDeviceFeatureOutputPositionWithDurationProperties {
  #[serde(skip_serializing_if = "Option::is_none")]
  position: Option<RangeInclusive<u32>>,
  #[serde(skip_serializing_if = "Option::is_none")]
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
    let position = RangeWithLimit::try_new(base.position().base(), &self.position)?;
    let duration = RangeWithLimit::try_new(base.duration().base(), &self.duration)?;
    Ok(
      ServerDeviceFeatureOutputPositionWithDurationProperties::new(
        &position,
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
      position: value.position().user().clone(),
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
  rotate_with_direction: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  oscillate: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  constrict: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  heater: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  led: Option<UserDeviceFeatureOutputValueProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  position: Option<UserDeviceFeatureOutputPositionProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  position_with_duration: Option<UserDeviceFeatureOutputPositionWithDurationProperties>,
  #[serde(skip_serializing_if = "Option::is_none")]
  spray: Option<UserDeviceFeatureOutputValueProperties>,
}

impl UserDeviceFeatureOutput {
  pub fn with_base_output(
    &self,
    base_output: &ServerDeviceFeatureOutput,
  ) -> Result<ServerDeviceFeatureOutput, ButtplugDeviceConfigError> {
    let mut output = ServerDeviceFeatureOutput::default();
    if let Some(base_vibrate) = base_output.vibrate() {
      if let Some(user_vibrate) = &self.vibrate {
        output.set_vibrate(Some(user_vibrate.with_base_properties(base_vibrate)?));
      } else {
        output.set_vibrate(base_output.vibrate().clone());
      }
    }
    if let Some(user_rotate) = &self.rotate {
      if let Some(base_rotate) = base_output.rotate() {
        output.set_rotate(Some(user_rotate.with_base_properties(base_rotate)?));
      } else {
        output.set_rotate(base_output.rotate().clone());
      }
    }
    if let Some(user_oscillate) = &self.oscillate {
      if let Some(base_oscillate) = base_output.oscillate() {
        output.set_oscillate(Some(user_oscillate.with_base_properties(base_oscillate)?));
      } else {
        output.set_oscillate(base_output.oscillate().clone());
      }
    }
    if let Some(user_constrict) = &self.constrict {
      if let Some(base_constrict) = base_output.constrict() {
        output.set_constrict(Some(user_constrict.with_base_properties(base_constrict)?));
      } else {
        output.set_constrict(base_output.constrict().clone());
      }
    }
    if let Some(user_heater) = &self.heater {
      if let Some(base_heater) = base_output.heater() {
        output.set_heater(Some(user_heater.with_base_properties(base_heater)?));
      } else {
        output.set_heater(base_output.heater().clone());
      }
    }
    if let Some(user_led) = &self.led {
      if let Some(base_led) = base_output.led() {
        output.set_led(Some(user_led.with_base_properties(base_led)?));
      } else {
        output.set_led(base_output.led().clone());
      }
    }
    if let Some(user_spray) = &self.spray {
      if let Some(base_spray) = base_output.spray() {
        output.set_spray(Some(user_spray.with_base_properties(base_spray)?));
      } else {
        output.set_spray(base_output.spray().clone());
      }
    }
    if let Some(user) = &self.position {
      if let Some(base) = base_output.position() {
        output.set_position(Some(user.with_base_properties(base)?));
      } else {
        output.set_position(base_output.position().clone());
      }
    }
    if let Some(user) = &self.position_with_duration {
      if let Some(base) = base_output.position_with_duration() {
        output.set_position_with_duration(Some(user.with_base_properties(base)?));
      } else {
        output.set_position_with_duration(base_output.position_with_duration().clone());
      }
    }
    Ok(output)
  }
}

impl From<&ServerDeviceFeatureOutput> for UserDeviceFeatureOutput {
  fn from(value: &ServerDeviceFeatureOutput) -> Self {
    Self {
      vibrate: value.vibrate().as_ref().map(|x| x.into()),
      rotate: value.rotate().as_ref().map(|x| x.into()),
      rotate_with_direction: value.rotate_with_direction().as_ref().map(|x| x.into()),
      oscillate: value.oscillate().as_ref().map(|x| x.into()),
      constrict: value.constrict().as_ref().map(|x| x.into()),
      heater: value.heater().as_ref().map(|x| x.into()),
      led: value.led().as_ref().map(|x| x.into()),
      position: value.position().as_ref().map(|x| x.into()),
      position_with_duration: value.position_with_duration().as_ref().map(|x| x.into()),
      spray: value.spray().as_ref().map(|x| x.into()),
    }
  }
}

#[derive(
  Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize,
)]
pub struct DeviceFeatureInputProperties {
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(serialize_with = "range_sequence_serialize")]
  value_range: Vec<RangeInclusive<i32>>,
  #[getset(get = "pub")]
  input_commands: HashSet<InputCommandType>,
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

impl Into<ServerDeviceFeatureInputProperties> for DeviceFeatureInputProperties {
  fn into(self) -> ServerDeviceFeatureInputProperties {
    ServerDeviceFeatureInputProperties::new(&self.value_range, &self.input_commands)
  }
}

#[derive(Clone, Debug, Default, Getters, Serialize, Deserialize)]
#[getset(get = "pub")]
pub struct DeviceFeatureInput {
  battery: Option<DeviceFeatureInputProperties>,
  rssi: Option<DeviceFeatureInputProperties>,
  pressure: Option<DeviceFeatureInputProperties>,
  button: Option<DeviceFeatureInputProperties>,
}

impl Into<ServerDeviceFeatureInput> for DeviceFeatureInput {
  fn into(self) -> ServerDeviceFeatureInput {
    let mut input = ServerDeviceFeatureInput::default();
    if let Some(battery) = self.battery {
      input.set_battery(Some(battery.into()));
    }
    if let Some(rssi) = self.rssi {
      input.set_rssi(Some(rssi.into()));
    }
    if let Some(pressure) = self.pressure {
      input.set_pressure(Some(pressure.into()));
    }
    if let Some(button) = self.button {
      input.set_button(Some(button.into()));
    }
    input
  }
}

#[derive(Clone, Debug, Default, Getters, Serialize, Deserialize, CopyGetters)]
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
  #[serde(skip_serializing_if = "BaseFeatureSettings::is_none", default)]
  feature_settings: BaseFeatureSettings,
}

impl Into<ServerDeviceFeature> for ConfigBaseDeviceFeature {
  fn into(self) -> ServerDeviceFeature {
    // This isn't resolving correctly using .and_then, so having to do it the long way?
    let output: Option<ServerDeviceFeatureOutput> = if let Some(o) = self.output {
      Some(o.into())
    } else {
      None
    };
    let input: Option<ServerDeviceFeatureInput> = if let Some(i) = self.input {
      Some(i.into())
    } else {
      None
    };
    ServerDeviceFeature::new(
      &self.description,
      self.id,
      None,
      self.feature_settings.alt_protocol_index,
      &output,
      &input,
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
        Some(o.with_base_output(&base)?)
      } else {
        None
      }
    } else {
      None
    };
    Ok(ServerDeviceFeature::new(
      &base_feature.description(),
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
      base_id: value.base_id().expect("Should have base id"),
      output: value.output().as_ref().map(|x| x.into()),
    }
  }
}
