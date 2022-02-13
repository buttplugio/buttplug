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

// Unlike other message components, MessageAttributes is always turned on for
// serialization, because it's used by device configuration files also.
//
// Also, unlike all other device messages, DeviceMessageAttributes are simply a
// message component. Since they're accessed via messages, and messages are
// immutable, we can leave the fields as public, versus trying to build
// accessors to everything.

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct DeviceMessageAttributes {
  #[serde(rename = "FeatureCount")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub feature_count: Option<u32>,
  #[serde(rename = "StepCount")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub step_count: Option<Vec<u32>>,
  #[serde(rename = "Endpoints")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub endpoints: Option<Vec<Endpoint>>,
  #[serde(rename = "MaxDuration")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_duration: Option<Vec<u32>>,
  #[serde(rename = "StepRange")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub step_range: Option<Vec<(u32, u32)>>,
  /*
  // Unimplemented attributes
  #[serde(rename = "Patterns")]
  #[serde(skip_serializing_if = "Option::is_none")]
  patterns: Option<Vec<Vec<String>>>,
  #[serde(rename = "ActuatorType")]
  #[serde(skip_serializing_if = "Option::is_none")]
  actuator_type: Option<Vec<String>>,
  */
  // Never serialize this, its for internal use only
  #[serde(rename = "FeatureOrder")]
  #[serde(skip_serializing)]
  pub feature_order: Option<Vec<u32>>,
}

impl DeviceMessageAttributes {
  fn check_feature_count_validity(&self, message_type: &ButtplugDeviceMessageType) -> Result<(), String> {
    info!("Feature count");
    if self.feature_count.is_none() {
      info!("Feature count error");
      Err(format!("Feature count is required for {}.", message_type))
    } else {
      Ok(())
    }
  }

  fn check_step_count(&self, message_type: &ButtplugDeviceMessageType) -> Result<(), String> {
    info!("Step count");
    if self.step_count.is_none() {
      Err(format!("Step count is required for {}.", message_type))
    } else if self.step_count.as_ref().expect("Checked").len() != *self.feature_count.as_ref().expect("Already checked in feature count check.") as usize {
      Err(format!("Step count array length must match feature count for {}.", message_type))
    } else {
      Ok(())
    }
  }

  fn check_step_range(&self, message_type: &ButtplugDeviceMessageType) -> Result<(), String> {
    info!("Step range: {:?}", self.step_range);
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
        info!("Checking message type");
        self.check_feature_count_validity(message_type)
          .and_then(|_| self.check_step_count(message_type))
          .and_then(|_| self.check_step_range(message_type))
          .and_then(|_| self.check_feature_order(message_type))
      },
      _ => Ok(())
    }.map_err(|error_str| ButtplugDeviceError::DeviceConfigurationFileError(error_str))
  }

  pub fn merge(&self, other: &DeviceMessageAttributes) -> DeviceMessageAttributes {
    DeviceMessageAttributes {
      feature_count: other.feature_count.or_else(|| self.feature_count),
      endpoints: other.endpoints.clone().or_else(|| self.endpoints.clone()),
      step_count: other.step_count.clone().or_else(|| self.step_count.clone()),
      max_duration: other.max_duration.clone().or_else(|| self.max_duration.clone()),
      step_range: other.step_range.clone().or_else(|| self.step_range.clone()),
      feature_order: other.feature_order.clone().or_else(|| self.feature_order.clone())
    }
  }
}