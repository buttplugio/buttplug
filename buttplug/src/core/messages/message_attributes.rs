// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::Endpoint;
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
  #[serde(skip)]
  pub feature_order: Option<Vec<u32>>,
  // Never serialize this, its for user config use only
  #[serde(rename = "StepRange")]
  #[serde(skip)]
  pub step_range: Option<Vec<(u32, u32)>>,
}
