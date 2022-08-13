// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::{
  errors::ButtplugDeviceError,
  messages::{ButtplugDeviceMessageType, Endpoint},
};
use getset::{Getters, Setters};
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActuatorType {
  Vibrate,
  // Single Direction Rotation Speed
  Rotate,
  Oscillate,
  Constrict,
  Inflate,
  // For instances where we specify a position to move to ASAP. Usually servos, probably for the
  // OSR-2/SR-6.
  Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)]
pub enum SensorType {
  Battery,
  RSSI,
  Button,
  Pressure,
  // Temperature,
  // Accelerometer,
  // Gyro,
}

// Unlike other message components, MessageAttributes is always turned on for
// serialization, because it's used by device configuration files also.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct DeviceMessageAttributes {
  // Generic commands
  #[getset(get = "pub")]
  #[serde(rename = "ScalarCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  scalar_cmd: Option<Vec<GenericDeviceMessageAttributes>>,
  #[getset(get = "pub")]
  #[serde(rename = "RotateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  rotate_cmd: Option<Vec<GenericDeviceMessageAttributes>>,
  #[getset(get = "pub")]
  #[serde(rename = "LinearCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  linear_cmd: Option<Vec<GenericDeviceMessageAttributes>>,

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

impl DeviceMessageAttributes {
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

  pub fn merge(&self, child: &DeviceMessageAttributes) -> DeviceMessageAttributes {
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
    let raw_attrs = RawDeviceMessageAttributes {
      endpoints: endpoints.clone().to_vec(),
    };
    self.raw_read_cmd = Some(raw_attrs.clone());
    self.raw_write_cmd = Some(raw_attrs.clone());
    self.raw_subscribe_cmd = Some(raw_attrs.clone());
  }
}

#[derive(Default)]
pub struct DeviceMessageAttributesBuilder {
  attrs: DeviceMessageAttributes,
}

impl DeviceMessageAttributesBuilder {
  pub fn scalar_cmd(&mut self, attrs: &Vec<GenericDeviceMessageAttributes>) -> &Self {
    self.attrs.scalar_cmd = Some(attrs.clone());
    self
  }

  pub fn rotate_cmd(&mut self, attrs: &Vec<GenericDeviceMessageAttributes>) -> &Self {
    self.attrs.rotate_cmd = Some(attrs.clone());
    self
  }

  pub fn linear_cmd(&mut self, attrs: &Vec<GenericDeviceMessageAttributes>) -> &Self {
    self.attrs.linear_cmd = Some(attrs.clone());
    self
  }

  pub fn sensor_read_cmd(&mut self, attrs: &Vec<SensorDeviceMessageAttributes>) -> &Self {
    self.attrs.sensor_read_cmd = Some(attrs.clone());
    self
  }

  pub fn sensor_subscribe_cmd(&mut self, attrs: &Vec<SensorDeviceMessageAttributes>) -> &Self {
    self.attrs.sensor_subscribe_cmd = Some(attrs.clone());
    self
  }

  pub fn raw_read_cmd(&mut self, endpoints: &Vec<Endpoint>) -> &Self {
    self.attrs.raw_read_cmd = Some(RawDeviceMessageAttributes::new(endpoints));
    self
  }

  pub fn raw_write_cmd(&mut self, endpoints: &Vec<Endpoint>) -> &Self {
    self.attrs.raw_write_cmd = Some(RawDeviceMessageAttributes::new(endpoints));
    self
  }

  pub fn raw_subscribe_cmd(&mut self, endpoints: &Vec<Endpoint>) -> &Self {
    self.attrs.raw_subscribe_cmd = Some(RawDeviceMessageAttributes::new(endpoints));
    self
  }

  pub fn finish(&self) -> DeviceMessageAttributes {
    self.attrs.clone()
  }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NullDeviceMessageAttributes {}

fn unspecified_feature() -> String {
  "No description available for feature".to_string()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct GenericDeviceMessageAttributes {
  #[getset(get = "pub")]
  #[serde(rename = "FeatureDescriptor")]
  #[serde(default = "unspecified_feature")]
  feature_descriptor: String,
  // This is the count we'll load from our config file, but we'll calculate it internally when
  // someone asks for it just in case we also have a step range.
  #[serde(rename = "StepCount")]
  step_count: u32,
  #[getset(get = "pub")]
  #[serde(rename = "ActuatorType")]
  actuator_type: ActuatorType,
  #[serde(rename = "StepRange")]
  #[serde(skip_serializing)]
  step_range: Option<RangeInclusive<u32>>,
}

impl GenericDeviceMessageAttributes {
  pub fn new(feature_descriptor: &str, step_count: u32, actuator_type: ActuatorType) -> Self {
    Self {
      feature_descriptor: feature_descriptor.to_owned(),
      step_count,
      actuator_type,
      step_range: None,
    }
  }

  pub fn step_count(&self) -> u32 {
    if let Some(range) = &self.step_range {
      range.end() - range.start()
    } else {
      self.step_count
    }
  }

  pub fn step_range(&self) -> RangeInclusive<u32> {
    if let Some(range) = &self.step_range {
      range.clone()
    } else {
      RangeInclusive::new(0, self.step_count)
    }
  }

  pub fn set_step_range(&mut self, range: &RangeInclusive<u32>) {
    self.step_range = Some(range.clone());
  }

  pub fn is_valid(
    &self,
    message_type: &ButtplugDeviceMessageType,
  ) -> Result<(), ButtplugDeviceError> {
    if let Some(step_range) = &self.step_range {
      // if step ranges are set up manually, they must be included for all acutators.
      if !step_range.contains(&self.step_count) {
        Err(ButtplugDeviceError::DeviceConfigurationError(format!(
          "Step range array values must have max value of step for {}.",
          message_type
        )))
      } else if step_range.is_empty() {
        Err(ButtplugDeviceError::DeviceConfigurationError(format!(
          "Step range out of order for {}, must be start <= x <= end.",
          message_type
        )))
      } else {
        Ok(())
      }
    } else {
      Ok(())
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, Getters, Setters)]
pub struct RawDeviceMessageAttributes {
  #[getset(get = "pub")]
  #[serde(rename = "Endpoints")]
  endpoints: Vec<Endpoint>,
}

impl RawDeviceMessageAttributes {
  pub fn new(endpoints: &Vec<Endpoint>) -> Self {
    Self {
      endpoints: endpoints.clone(),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct SensorDeviceMessageAttributes {
  #[getset(get = "pub")]
  #[serde(rename = "FeatureDescriptor")]
  feature_descriptor: String,
  #[getset(get = "pub")]
  #[serde(rename = "SensorType")]
  sensor_type: SensorType,
  #[getset(get = "pub")]
  #[serde(rename = "SensorRange")]
  sensor_range: [u32; 2],
}

/*
impl SensorDeviceMessageAttributes {
  pub fn new(feature_descriptor: &str, sensor_type: SensorType) -> Self {
    Self { feature_descriptor: feature_descriptor.to_owned(), sensor_type }
  }
}
 */

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct DeviceMessageAttributesV2 {
  // Generic commands
  #[getset(get = "pub")]
  #[serde(rename = "VibrateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  vibrate_cmd: Option<GenericDeviceMessageAttributesV2>,
  #[getset(get = "pub")]
  #[serde(rename = "RotateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  rotate_cmd: Option<GenericDeviceMessageAttributesV2>,
  #[getset(get = "pub")]
  #[serde(rename = "LinearCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  linear_cmd: Option<GenericDeviceMessageAttributesV2>,
  #[getset(get = "pub")]
  #[serde(rename = "BatteryLevelCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  battery_level_cmd: Option<NullDeviceMessageAttributes>,

  // RSSILevel is added post-serialization (only for bluetooth devices)
  #[getset(get = "pub")]
  #[serde(rename = "RSSILevelCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  rssi_level_cmd: Option<NullDeviceMessageAttributes>,

  // StopDeviceCmd always exists
  #[getset(get = "pub")]
  #[serde(rename = "StopDeviceCmd")]
  stop_device_cmd: NullDeviceMessageAttributes,

  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(rename = "RawReadCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  raw_read_cmd: Option<RawDeviceMessageAttributes>,
  #[getset(get = "pub")]
  #[serde(rename = "RawWriteCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  raw_write_cmd: Option<RawDeviceMessageAttributes>,
  #[getset(get = "pub")]
  #[serde(rename = "RawSubscribeCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  raw_subscribe_cmd: Option<RawDeviceMessageAttributes>,

  // Needed to load from config for fallback, but unused here.
  #[getset(get = "pub")]
  #[serde(rename = "FleshlightLaunchFW12Cmd")]
  #[serde(skip)]
  fleshlight_launch_fw12_cmd: Option<NullDeviceMessageAttributes>,
  #[getset(get = "pub")]
  #[serde(rename = "VorzeA10CycloneCmd")]
  #[serde(skip)]
  vorze_a10_cyclone_cmd: Option<NullDeviceMessageAttributes>,
}

impl From<DeviceMessageAttributes> for DeviceMessageAttributesV2 {
  fn from(other: DeviceMessageAttributes) -> Self {
    Self {
      vibrate_cmd: other
        .scalar_cmd()
        .as_ref()
        .map(|x| GenericDeviceMessageAttributesV2::vibrate_cmd_from_scalar_cmd(x.clone())),
      rotate_cmd: other
        .rotate_cmd()
        .as_ref()
        .map(|x| GenericDeviceMessageAttributesV2::from(x.clone())),
      linear_cmd: other
        .linear_cmd()
        .as_ref()
        .map(|x| GenericDeviceMessageAttributesV2::from(x.clone())),
      battery_level_cmd: {
        if let Some(sensor_info) = &other.sensor_read_cmd {
          if sensor_info
            .iter()
            .any(|x| *x.sensor_type() == SensorType::Battery)
          {
            Some(NullDeviceMessageAttributes::default())
          } else {
            None
          }
        } else {
          None
        }
      },
      rssi_level_cmd: {
        if let Some(sensor_info) = &other.sensor_read_cmd {
          if sensor_info
            .iter()
            .any(|x| *x.sensor_type() == SensorType::RSSI)
          {
            Some(NullDeviceMessageAttributes::default())
          } else {
            None
          }
        } else {
          None
        }
      },
      stop_device_cmd: other.stop_device_cmd().clone(),
      raw_read_cmd: other.raw_read_cmd().clone(),
      raw_write_cmd: other.raw_write_cmd().clone(),
      raw_subscribe_cmd: other.raw_subscribe_cmd().clone(),
      fleshlight_launch_fw12_cmd: other.fleshlight_launch_fw12_cmd().clone(),
      vorze_a10_cyclone_cmd: other.vorze_a10_cyclone_cmd().clone(),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct GenericDeviceMessageAttributesV2 {
  #[getset(get = "pub")]
  #[serde(rename = "FeatureCount")]
  feature_count: u32,
  #[getset(get = "pub")]
  #[serde(rename = "StepCount")]
  step_count: Vec<u32>,
}

impl GenericDeviceMessageAttributesV2 {
  pub fn vibrate_cmd_from_scalar_cmd(attributes_vec: Vec<GenericDeviceMessageAttributes>) -> Self {
    let mut feature_count = 0u32;
    let mut step_count = vec![];
    for attr in attributes_vec {
      if *attr.actuator_type() == ActuatorType::Vibrate {
        feature_count += 1;
        step_count.push(attr.step_count());
      }
    }
    Self {
      feature_count,
      step_count,
    }
  }
}

impl From<Vec<GenericDeviceMessageAttributes>> for GenericDeviceMessageAttributesV2 {
  fn from(attributes_vec: Vec<GenericDeviceMessageAttributes>) -> Self {
    Self {
      feature_count: attributes_vec.len() as u32,
      step_count: attributes_vec.iter().map(|x| x.step_count()).collect(),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct DeviceMessageAttributesV1 {
  // Generic commands
  #[getset(get = "pub")]
  #[serde(rename = "VibrateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  vibrate_cmd: Option<GenericDeviceMessageAttributesV1>,
  #[getset(get = "pub")]
  #[serde(rename = "RotateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  rotate_cmd: Option<GenericDeviceMessageAttributesV1>,
  #[getset(get = "pub")]
  #[serde(rename = "LinearCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  linear_cmd: Option<GenericDeviceMessageAttributesV1>,

  // StopDeviceCmd always exists
  #[getset(get = "pub")]
  stop_device_cmd: NullDeviceMessageAttributes,

  // Obsolete commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  single_motor_vibrate_cmd: Option<NullDeviceMessageAttributes>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  fleshlight_launch_fw12_cmd: Option<NullDeviceMessageAttributes>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  vorze_a10_cyclone_cmd: Option<NullDeviceMessageAttributes>,
}

impl From<DeviceMessageAttributesV2> for DeviceMessageAttributesV1 {
  fn from(other: DeviceMessageAttributesV2) -> Self {
    Self {
      vibrate_cmd: other
        .vibrate_cmd()
        .as_ref()
        .map(|x| GenericDeviceMessageAttributesV1::from(x.clone())),
      rotate_cmd: other
        .rotate_cmd()
        .as_ref()
        .map(|x| GenericDeviceMessageAttributesV1::from(x.clone())),
      linear_cmd: other
        .linear_cmd()
        .as_ref()
        .map(|x| GenericDeviceMessageAttributesV1::from(x.clone())),
      stop_device_cmd: other.stop_device_cmd().clone(),
      fleshlight_launch_fw12_cmd: other.fleshlight_launch_fw12_cmd().clone(),
      vorze_a10_cyclone_cmd: other.vorze_a10_cyclone_cmd().clone(),
      single_motor_vibrate_cmd: if other.vibrate_cmd().is_some() {
        Some(NullDeviceMessageAttributes::default())
      } else {
        None
      },
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct GenericDeviceMessageAttributesV1 {
  #[serde(rename = "FeatureCount")]
  feature_count: u32,
}

impl From<GenericDeviceMessageAttributesV2> for GenericDeviceMessageAttributesV1 {
  fn from(attributes: GenericDeviceMessageAttributesV2) -> Self {
    Self {
      feature_count: *attributes.feature_count(),
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  pub fn test_step_count_calculation() {
    let mut vibrate_attributes =
      GenericDeviceMessageAttributes::new("test", 10, ActuatorType::Vibrate);
    assert_eq!(vibrate_attributes.step_count(), 10);
    vibrate_attributes.set_step_range(&RangeInclusive::new(3u32, 7));
    assert_eq!(vibrate_attributes.step_count(), 4);
  }
}
