// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use getset::{Getters, Setters};
use serde::{Deserialize, Serialize};

use crate::core::message::DeviceFeature;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NullDeviceMessageAttributesV1 {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct ClientDeviceMessageAttributesV1 {
  // Generic commands
  #[getset(get = "pub")]
  #[serde(rename = "VibrateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::core::message) vibrate_cmd: Option<GenericDeviceMessageAttributesV1>,
  #[getset(get = "pub")]
  #[serde(rename = "RotateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::core::message) rotate_cmd: Option<GenericDeviceMessageAttributesV1>,
  #[getset(get = "pub")]
  #[serde(rename = "LinearCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::core::message) linear_cmd: Option<GenericDeviceMessageAttributesV1>,

  // StopDeviceCmd always exists
  #[getset(get = "pub")]
  pub(in crate::core::message) stop_device_cmd: NullDeviceMessageAttributesV1,

  // Obsolete commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::core::message) single_motor_vibrate_cmd: Option<NullDeviceMessageAttributesV1>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::core::message) fleshlight_launch_fw12_cmd: Option<NullDeviceMessageAttributesV1>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::core::message) vorze_a10_cyclone_cmd: Option<NullDeviceMessageAttributesV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct GenericDeviceMessageAttributesV1 {
  #[serde(rename = "FeatureCount")]
  feature_count: u32,
  #[getset(get = "pub")]
  #[serde(skip)]
  pub(in crate::core::message) features: Vec<DeviceFeature>,
}

impl GenericDeviceMessageAttributesV1 {
  pub fn new(feature_count: u32, features: &Vec<DeviceFeature>) -> Self {
    Self {
      feature_count,
      features: features.clone(),
    }
  }
}
