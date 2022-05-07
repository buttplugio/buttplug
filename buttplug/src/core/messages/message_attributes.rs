// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    messages::ButtplugDeviceMessageType,
  },
  device::Endpoint,
};
use serde::{Deserialize, Serialize};
use getset::{Getters, Setters};

#[derive(Debug, Default)]
pub struct DeviceMessageAttributesBuilder {
  attributes: DeviceMessageAttributes
}

impl DeviceMessageAttributesBuilder {
  pub fn feature_count(mut self, count: u32) -> DeviceMessageAttributesBuilder {
    self.attributes.feature_count = Some(count);
    self
  }

  pub fn step_count(mut self, step_count: Vec<u32>) -> DeviceMessageAttributesBuilder {
    self.attributes.step_count = Some(step_count);
    self
  }

  pub fn step_range(mut self, step_range: Vec<(u32, u32)>) -> DeviceMessageAttributesBuilder {
    self.attributes.step_range = Some(step_range);
    self
  }

  pub fn endpoints(mut self, endpoints: Vec<Endpoint>) ->  DeviceMessageAttributesBuilder {
    self.attributes.endpoints = Some(endpoints);
    self
  }

  pub fn max_duration(mut self, max_duration: Vec<u32>) ->  DeviceMessageAttributesBuilder {
    self.attributes.max_duration = Some(max_duration);
    self
  }

  pub fn feature_order(mut self, feature_order: Vec<u32>)  ->  DeviceMessageAttributesBuilder {
    self.attributes.feature_order = Some(feature_order);
    self
  }

  pub fn build(self, message_type: &ButtplugDeviceMessageType) -> Result<DeviceMessageAttributes, ButtplugDeviceError> {
    self.attributes.check(&message_type)?;
    Ok(self.attributes)
  }

  // Required if we want to use this to build messages for v0/v1 fallbacks without creating specific
  // attribute version structs for those fallbacks.
  //
  // TODO Look at creating versioned structs for this in v3
  pub fn build_without_check(self) -> DeviceMessageAttributes {
    self.attributes
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActuatorType {
  Vibrate,
  Rotate,
  Linear,
  Oscillation,
  Constrict,
  Inflate
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SensorType {
  Button,
  Pressure,
  RSSI,
  Battery,
  // Accelerometer,
  // Gyro,
}

// Unlike other message components, MessageAttributes is always turned on for
// serialization, because it's used by device configuration files also.
//
// Also, unlike all other device messages, DeviceMessageAttributes are simply a
// message component. Since they're accessed via messages, and messages are
// immutable, we can leave the fields as public, versus trying to build
// accessors to everything.

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize, Getters, Setters)]
pub struct DeviceMessageAttributes {
  #[getset(get="pub")]
  #[serde(rename = "FeatureCount")]
  #[serde(skip_serializing_if = "Option::is_none")]
  feature_count: Option<u32>,
  #[serde(rename = "FeatureDescriptors")]
  #[serde(skip_serializing_if = "Option::is_none")]
  feature_descriptors: Option<Vec<String>>,
  #[serde(rename = "StepCount")]
  #[serde(skip_serializing_if = "Option::is_none")]
  step_count: Option<Vec<u32>>,
  #[getset(get="pub")]
  #[serde(rename = "Endpoints")]
  #[serde(skip_serializing_if = "Option::is_none")]
  endpoints: Option<Vec<Endpoint>>,
  #[getset(get="pub")]
  #[serde(rename = "MaxDuration")]
  #[serde(skip_serializing_if = "Option::is_none")]
  max_duration: Option<Vec<u32>>,
  #[getset(get="pub")]
  #[serde(rename = "ActuatorType")]
  #[serde(skip_serializing_if = "Option::is_none")]
  actuator_type: Option<Vec<ActuatorType>>,
  #[getset(get="pub")]
  #[serde(rename = "ActuatorType")]
  #[serde(skip_serializing_if = "Option::is_none")]
  sensor_type: Option<Vec<SensorType>>,
  // Never serialize this, its for internal use only
  #[getset(get="pub")]
  #[serde(rename = "StepRange")]
  #[serde(skip_serializing_if = "Option::is_none")]
  step_range: Option<Vec<(u32, u32)>>,
  // Never serialize this, its for internal use only
  #[getset(get="pub")]
  #[serde(rename = "FeatureOrder")]
  #[serde(skip)]
  feature_order: Option<Vec<u32>>,
  /*
  // Unimplemented attributes
  #[serde(rename = "Patterns")]
  #[serde(skip_serializing_if = "Option::is_none")]
  patterns: Option<Vec<Vec<String>>>,
  */
}

impl DeviceMessageAttributes {
  pub fn step_count(&self) -> Option<Vec<u32>> {
    if let Some(ranges) = &self.step_range {
      let mut step_range = vec!();
      for range in ranges {
        step_range.push(range.1 - range.0);
      }
      Some(step_range)
    } else {
      self.step_count.clone()
    }
  }

  fn check_feature_count_validity(&self, message_type: &ButtplugDeviceMessageType) -> Result<(), String> {
    if self.feature_count.is_none() {
      info!("Feature count error");
      Err(format!("Feature count is required for {}.", message_type))
    } else {
      Ok(())
    }
  }

  fn check_step_count(&self, message_type: &ButtplugDeviceMessageType) -> Result<(), String> {
    if self.step_count.is_none() {
      Err(format!("Step count is required for {}.", message_type))
    } else if self.step_count.as_ref().expect("Checked").len() != *self.feature_count.as_ref().expect("Already checked in feature count check.") as usize {
      Err(format!("Step count array length must match feature count for {}.", message_type))
    } else {
      Ok(())
    }
  }

  fn check_step_range(&self, message_type: &ButtplugDeviceMessageType) -> Result<(), String> {
    if let Some(step_range) = &self.step_range {
      // if step ranges are set up manually, they must be included for all acutators.
      if step_range.len() != *self.feature_count.as_ref().expect("Already checked in feature count check.") as usize {
        Err(format!("Step range array length must match feature count for {}.", message_type))
      } else if step_range.iter().any(|range| { info!("{:?}", range); range.1 <= range.0 }) {
        Err(format!("Step range array values must have an increasing range for {}.", message_type))
      } else if step_range.iter().enumerate().any(|(index, range)| range.1 > self.step_count.as_ref().expect("Already checked in step count check")[index]) {
        Err(format!("Step range array values must have max value of step for {}.", message_type))
      } else {
        Ok(())
      }
    } else {
      Ok(())
    }
  }

  fn check_feature_order(&self, message_type: &ButtplugDeviceMessageType) -> Result<(), String> {
    if let Some(feature_order) = &self.feature_order {
      if feature_order.len() != *self.feature_count.as_ref().expect("Already checked in feature count check.") as usize {
        Err(format!("Feature order must have the same number of elements as feature count for {}.", message_type))
      } else {
        Ok(())
      }
    } else {
      Ok(())
    }
  }

  // Check things that the JSON schema doesn't check. This is mostly for fields that reference other
  // fields within device attributes.
  //
  // Things checked automatically we can leave out here:
  // - Lots of stuff in the JSON Schema
  // - Validity of endpoints in RawMessages (via Serde deserialization)
  // - Validity of Acutator Types (via Serde deserialization)
  pub fn check(&self, message_type: &ButtplugDeviceMessageType) -> Result<(), ButtplugDeviceError> {
    match message_type {
      ButtplugDeviceMessageType::VibrateCmd | ButtplugDeviceMessageType::RotateCmd | ButtplugDeviceMessageType::LinearCmd => {
        self.check_feature_count_validity(message_type)
          .and_then(|_| self.check_step_count(message_type))
          .and_then(|_| self.check_step_range(message_type))
          .and_then(|_| self.check_feature_order(message_type))
      },
      ButtplugDeviceMessageType::RawReadCmd | ButtplugDeviceMessageType::RawWriteCmd | ButtplugDeviceMessageType::RawSubscribeCmd | ButtplugDeviceMessageType::RawUnsubscribeCmd => {
        self.endpoints.is_some().then(|| ()).ok_or(format!("Endpoints vector must exist for {}.", message_type))
      },
      _ => Ok(())
    }.map_err(ButtplugDeviceError::DeviceConfigurationError)
  }

  pub fn merge(&self, other: &DeviceMessageAttributes) -> DeviceMessageAttributes {
    DeviceMessageAttributes {
      feature_count: other.feature_count.or(self.feature_count),
      feature_descriptors: other.feature_descriptors.clone().or_else(|| self.feature_descriptors.clone()),
      actuator_type: other.actuator_type.clone().or_else(|| self.actuator_type.clone()),
      sensor_type: other.sensor_type.clone().or_else(|| self.sensor_type.clone()),
      endpoints: other.endpoints.clone().or_else(|| self.endpoints.clone()),
      step_count: other.step_count.clone().or_else(|| self.step_count.clone()),
      max_duration: other.max_duration.clone().or_else(|| self.max_duration.clone()),
      step_range: other.step_range.clone().or_else(|| self.step_range.clone()),
      feature_order: other.feature_order.clone().or_else(|| self.feature_order.clone())
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  pub fn test_step_count_calculation() {
    let vibrate_attributes = DeviceMessageAttributesBuilder::default()
      .feature_count(2)
      .step_count(vec![20, 20])
      .step_range(vec![(10, 15), (10, 20)])
      .build(&ButtplugDeviceMessageType::VibrateCmd)
      .unwrap();

    assert_eq!(vibrate_attributes.step_count(), Some(vec![5, 10]))
  }
}