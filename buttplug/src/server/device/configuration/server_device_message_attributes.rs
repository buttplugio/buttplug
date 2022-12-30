use std::ops::RangeInclusive;

use getset::{Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};

use crate::core::{
  errors::ButtplugDeviceError,
  message::{
    ActuatorType,
    ButtplugDeviceMessageType,
    ClientDeviceMessageAttributes,
    ClientDeviceMessageAttributesBuilder,
    ClientGenericDeviceMessageAttributes,
    Endpoint,
    NullDeviceMessageAttributes,
    RawDeviceMessageAttributes,
    SensorDeviceMessageAttributes,
    SensorType,
  },
};

// Unlike other message components, MessageAttributes is always turned on for
// serialization, because it's used by device configuration files also.
#[derive(
  Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Getters, MutGetters, Setters,
)]
pub struct ServerDeviceMessageAttributes {
  // Generic commands
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "ScalarCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  scalar_cmd: Option<Vec<ServerGenericDeviceMessageAttributes>>,
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "RotateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  rotate_cmd: Option<Vec<ServerGenericDeviceMessageAttributes>>,
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "LinearCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  linear_cmd: Option<Vec<ServerGenericDeviceMessageAttributes>>,

  // Sensor Messages
  #[getset(get = "pub")]
  #[serde(rename = "SensorReadCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  sensor_read_cmd: Option<Vec<SensorDeviceMessageAttributes>>,
  #[getset(get = "pub")]
  #[serde(rename = "SensorSubscribeCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  sensor_subscribe_cmd: Option<Vec<SensorDeviceMessageAttributes>>,

  // StopDeviceCmd always exists
  #[getset(get = "pub")]
  #[serde(rename = "StopDeviceCmd")]
  #[serde(skip_deserializing)]
  stop_device_cmd: NullDeviceMessageAttributes,

  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(rename = "RawReadCmd")]
  #[serde(skip_deserializing)]
  #[serde(skip_serializing_if = "Option::is_none")]
  raw_read_cmd: Option<RawDeviceMessageAttributes>,
  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(rename = "RawWriteCmd")]
  #[serde(skip_deserializing)]
  #[serde(skip_serializing_if = "Option::is_none")]
  raw_write_cmd: Option<RawDeviceMessageAttributes>,
  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(rename = "RawSubscribeCmd")]
  #[serde(skip_deserializing)]
  #[serde(skip_serializing_if = "Option::is_none")]
  raw_subscribe_cmd: Option<RawDeviceMessageAttributes>,

  // Needed to load from config for fallback, but unused here.
  #[getset(get = "pub")]
  #[serde(rename = "FleshlightLaunchFW12Cmd")]
  #[serde(skip_serializing)]
  fleshlight_launch_fw12_cmd: Option<NullDeviceMessageAttributes>,
  #[getset(get = "pub")]
  #[serde(rename = "VorzeA10CycloneCmd")]
  #[serde(skip_serializing)]
  vorze_a10_cyclone_cmd: Option<NullDeviceMessageAttributes>,
}

impl ServerDeviceMessageAttributes {
  pub fn raw_unsubscribe_cmd(&self) -> &Option<RawDeviceMessageAttributes> {
    self.raw_subscribe_cmd()
  }

  pub fn message_allowed(&self, message_type: &ButtplugDeviceMessageType) -> bool {
    match message_type {
      ButtplugDeviceMessageType::ScalarCmd => self.scalar_cmd.is_some(),
      // VibrateCmd and SingleMotorVibrateCmd will derive from Scalars, so errors will be thrown in
      // the scalar parser if the actuator isn't correct.
      ButtplugDeviceMessageType::VibrateCmd => self.scalar_cmd.is_some(),
      ButtplugDeviceMessageType::SingleMotorVibrateCmd => self.scalar_cmd.is_some(),
      ButtplugDeviceMessageType::SensorReadCmd => self.sensor_read_cmd.is_some(),
      ButtplugDeviceMessageType::SensorSubscribeCmd => self.sensor_subscribe_cmd.is_some(),
      ButtplugDeviceMessageType::SensorUnsubscribeCmd => self.sensor_subscribe_cmd.is_some(),
      ButtplugDeviceMessageType::LinearCmd => self.linear_cmd.is_some(),
      ButtplugDeviceMessageType::RotateCmd => self.rotate_cmd.is_some(),
      ButtplugDeviceMessageType::BatteryLevelCmd => {
        if let Some(sensor_info) = &self.sensor_read_cmd {
          sensor_info
            .iter()
            .any(|x| *x.sensor_type() == SensorType::Battery)
        } else {
          false
        }
      }
      ButtplugDeviceMessageType::FleshlightLaunchFW12Cmd => {
        self.fleshlight_launch_fw12_cmd.is_some()
      }
      ButtplugDeviceMessageType::RSSILevelCmd => {
        if let Some(sensor_info) = &self.sensor_read_cmd {
          sensor_info
            .iter()
            .any(|x| *x.sensor_type() == SensorType::RSSI)
        } else {
          false
        }
      }
      ButtplugDeviceMessageType::RawReadCmd => self.raw_read_cmd.is_some(),
      ButtplugDeviceMessageType::RawSubscribeCmd => self.raw_subscribe_cmd.is_some(),
      ButtplugDeviceMessageType::RawUnsubscribeCmd => self.raw_subscribe_cmd.is_some(),
      ButtplugDeviceMessageType::RawWriteCmd => self.raw_write_cmd.is_some(),
      ButtplugDeviceMessageType::VorzeA10CycloneCmd => self.vorze_a10_cyclone_cmd.is_some(),
      ButtplugDeviceMessageType::StopDeviceCmd => true,
      ButtplugDeviceMessageType::KiirooCmd => false,
      ButtplugDeviceMessageType::LovenseCmd => false,
    }
  }

  pub fn merge(&self, child: &ServerDeviceMessageAttributes) -> ServerDeviceMessageAttributes {
    Self {
      rotate_cmd: child
        .rotate_cmd()
        .clone()
        .or_else(|| self.rotate_cmd().clone()),
      linear_cmd: child
        .linear_cmd()
        .clone()
        .or_else(|| self.linear_cmd().clone()),
      scalar_cmd: child
        .scalar_cmd()
        .clone()
        .or_else(|| self.scalar_cmd().clone()),
      sensor_read_cmd: child
        .sensor_read_cmd()
        .clone()
        .or_else(|| self.sensor_read_cmd().clone()),
      sensor_subscribe_cmd: child
        .sensor_subscribe_cmd()
        .clone()
        .or_else(|| self.sensor_subscribe_cmd().clone()),
      stop_device_cmd: NullDeviceMessageAttributes::default(),
      raw_read_cmd: child
        .raw_read_cmd()
        .clone()
        .or_else(|| self.raw_read_cmd().clone()),
      raw_write_cmd: child
        .raw_write_cmd()
        .clone()
        .or_else(|| self.raw_write_cmd().clone()),
      raw_subscribe_cmd: child
        .raw_subscribe_cmd()
        .clone()
        .or_else(|| self.raw_subscribe_cmd().clone()),
      fleshlight_launch_fw12_cmd: child
        .fleshlight_launch_fw12_cmd()
        .clone()
        .or_else(|| self.fleshlight_launch_fw12_cmd().clone()),
      vorze_a10_cyclone_cmd: child
        .vorze_a10_cyclone_cmd()
        .clone()
        .or_else(|| self.vorze_a10_cyclone_cmd().clone()),
    }
  }

  pub fn add_raw_messages(&mut self, endpoints: &[Endpoint]) {
    let raw_attrs = RawDeviceMessageAttributes::new(endpoints);
    self.raw_read_cmd = Some(raw_attrs.clone());
    self.raw_write_cmd = Some(raw_attrs.clone());
    self.raw_subscribe_cmd = Some(raw_attrs);
  }
}

impl From<ServerDeviceMessageAttributes> for ClientDeviceMessageAttributes {
  fn from(attrs: ServerDeviceMessageAttributes) -> Self {
    let mut builder = ClientDeviceMessageAttributesBuilder::default();
    if let Some(scalar_cmd) = attrs.scalar_cmd {
      let commands: Vec<ClientGenericDeviceMessageAttributes> =
        scalar_cmd.iter().cloned().map(|x| x.into()).collect();
      builder.scalar_cmd(&commands);
    }
    if let Some(rotate_cmd) = attrs.rotate_cmd {
      let commands: Vec<ClientGenericDeviceMessageAttributes> =
        rotate_cmd.iter().cloned().map(|x| x.into()).collect();
      builder.rotate_cmd(&commands);
    }
    if let Some(linear_cmd) = attrs.linear_cmd {
      let commands: Vec<ClientGenericDeviceMessageAttributes> =
        linear_cmd.iter().cloned().map(|x| x.into()).collect();
      builder.linear_cmd(&commands);
    }
    if let Some(sensor_read_cmd) = attrs.sensor_read_cmd {
      builder.sensor_read_cmd(&sensor_read_cmd);
    }
    if let Some(sensor_subscribe_cmd) = attrs.sensor_subscribe_cmd {
      builder.sensor_subscribe_cmd(&sensor_subscribe_cmd);
    }
    if let Some(raw_read_cmd) = attrs.raw_read_cmd {
      builder.raw_read_cmd(raw_read_cmd.endpoints());
    }
    if let Some(raw_write_cmd) = attrs.raw_write_cmd {
      builder.raw_write_cmd(raw_write_cmd.endpoints());
    }
    if let Some(raw_subscribe_cmd) = attrs.raw_subscribe_cmd {
      builder.raw_subscribe_cmd(raw_subscribe_cmd.endpoints());
    }
    builder.finish()
  }
}

#[derive(Default)]
pub struct ServerDeviceMessageAttributesBuilder {
  attrs: ServerDeviceMessageAttributes,
}

impl ServerDeviceMessageAttributesBuilder {
  pub fn scalar_cmd(&mut self, attrs: &[ServerGenericDeviceMessageAttributes]) -> &Self {
    self.attrs.scalar_cmd = Some(attrs.to_vec());
    self
  }

  pub fn rotate_cmd(&mut self, attrs: &[ServerGenericDeviceMessageAttributes]) -> &Self {
    self.attrs.rotate_cmd = Some(attrs.to_vec());
    self
  }

  pub fn linear_cmd(&mut self, attrs: &[ServerGenericDeviceMessageAttributes]) -> &Self {
    self.attrs.linear_cmd = Some(attrs.to_vec());
    self
  }

  pub fn sensor_read_cmd(&mut self, attrs: &[SensorDeviceMessageAttributes]) -> &Self {
    self.attrs.sensor_read_cmd = Some(attrs.to_vec());
    self
  }

  pub fn sensor_subscribe_cmd(&mut self, attrs: &[SensorDeviceMessageAttributes]) -> &Self {
    self.attrs.sensor_subscribe_cmd = Some(attrs.to_vec());
    self
  }

  pub fn raw_read_cmd(&mut self, endpoints: &[Endpoint]) -> &Self {
    self.attrs.raw_read_cmd = Some(RawDeviceMessageAttributes::new(endpoints));
    self
  }

  pub fn raw_write_cmd(&mut self, endpoints: &[Endpoint]) -> &Self {
    self.attrs.raw_write_cmd = Some(RawDeviceMessageAttributes::new(endpoints));
    self
  }

  pub fn raw_subscribe_cmd(&mut self, endpoints: &[Endpoint]) -> &Self {
    self.attrs.raw_subscribe_cmd = Some(RawDeviceMessageAttributes::new(endpoints));
    self
  }

  pub fn finish(&self) -> ServerDeviceMessageAttributes {
    self.attrs.clone()
  }
}

fn unspecified_feature() -> String {
  "N/A".to_string()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct ServerGenericDeviceMessageAttributes {
  #[getset(get = "pub")]
  #[serde(rename = "FeatureDescriptor")]
  #[serde(default = "unspecified_feature")]
  feature_descriptor: String,
  #[getset(get = "pub")]
  #[serde(rename = "ActuatorType")]
  actuator_type: ActuatorType,
  #[serde(rename = "StepRange")]
  #[serde(skip_serializing)]
  #[getset(get = "pub", set = "pub")]
  step_range: RangeInclusive<u32>,
}

impl From<ServerGenericDeviceMessageAttributes> for ClientGenericDeviceMessageAttributes {
  fn from(attrs: ServerGenericDeviceMessageAttributes) -> Self {
    ClientGenericDeviceMessageAttributes::new(
      &attrs.feature_descriptor,
      attrs.step_count(),
      attrs.actuator_type,
    )
  }
}

impl ServerGenericDeviceMessageAttributes {
  pub fn new(
    feature_descriptor: &str,
    step_range: &RangeInclusive<u32>,
    actuator_type: ActuatorType,
  ) -> Self {
    Self {
      feature_descriptor: feature_descriptor.to_owned(),
      actuator_type,
      step_range: step_range.clone(),
    }
  }

  pub fn step_count(&self) -> u32 {
    self.step_range.end() - self.step_range.start()
  }

  pub fn is_valid(
    &self,
    message_type: &ButtplugDeviceMessageType,
  ) -> Result<(), ButtplugDeviceError> {
    if self.step_range.is_empty() {
      Err(ButtplugDeviceError::DeviceConfigurationError(format!(
        "Step range out of order for {}, must be start <= x <= end.",
        message_type
      )))
    } else {
      Ok(())
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  pub fn test_step_count_calculation() {
    let mut vibrate_attributes = ServerGenericDeviceMessageAttributes::new(
      "test",
      &RangeInclusive::new(0, 10),
      ActuatorType::Vibrate,
    );
    assert_eq!(vibrate_attributes.step_count(), 10);
    vibrate_attributes.set_step_range(RangeInclusive::new(3u32, 7));
    assert_eq!(vibrate_attributes.step_count(), 4);
  }
}
