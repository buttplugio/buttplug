// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  v1::{
    ClientDeviceMessageAttributesV1,
    GenericDeviceMessageAttributesV1,
    NullDeviceMessageAttributesV1,
  },
  v3::ClientDeviceMessageAttributesV3,
};
use buttplug_core::message::DeviceFeature;
use getset::{CopyGetters, Getters, Setters};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct ClientDeviceMessageAttributesV2 {
  // Generic commands
  #[getset(get = "pub")]
  #[serde(rename = "VibrateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::message) vibrate_cmd: Option<GenericDeviceMessageAttributesV2>,
  #[getset(get = "pub")]
  #[serde(rename = "RotateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::message) rotate_cmd: Option<GenericDeviceMessageAttributesV2>,
  #[getset(get = "pub")]
  #[serde(rename = "LinearCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::message) linear_cmd: Option<GenericDeviceMessageAttributesV2>,
  #[getset(get = "pub")]
  #[serde(rename = "BatteryLevelCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::message) battery_level_cmd: Option<NullDeviceMessageAttributesV1>,

  // RSSILevel is added post-serialization (only for bluetooth devices)
  #[getset(get = "pub")]
  #[serde(rename = "RSSILevelCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::message) rssi_level_cmd: Option<NullDeviceMessageAttributesV1>,

  // StopDeviceCmd always exists
  #[getset(get = "pub")]
  #[serde(rename = "StopDeviceCmd")]
  pub(in crate::message) stop_device_cmd: NullDeviceMessageAttributesV1,

  // Needed to load from config for fallback, but unused here.
  #[getset(get = "pub")]
  #[serde(rename = "FleshlightLaunchFW12Cmd")]
  #[serde(skip)]
  pub(in crate::message) fleshlight_launch_fw12_cmd: Option<NullDeviceMessageAttributesV1>,
  #[getset(get = "pub")]
  #[serde(rename = "VorzeA10CycloneCmd")]
  #[serde(skip)]
  pub(in crate::message) vorze_a10_cyclone_cmd: Option<NullDeviceMessageAttributesV1>,
}

impl From<ClientDeviceMessageAttributesV2> for ClientDeviceMessageAttributesV1 {
  fn from(other: ClientDeviceMessageAttributesV2) -> Self {
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
        Some(NullDeviceMessageAttributesV1::default())
      } else {
        None
      },
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, CopyGetters, Setters)]
pub struct GenericDeviceMessageAttributesV2 {
  #[getset(get_copy = "pub")]
  #[serde(rename = "FeatureCount")]
  pub(in crate::message) feature_count: u32,
  #[getset(get = "pub")]
  #[serde(rename = "StepCount")]
  pub(in crate::message) step_count: Vec<u32>,
}

impl From<GenericDeviceMessageAttributesV2> for GenericDeviceMessageAttributesV1 {
  fn from(attributes: GenericDeviceMessageAttributesV2) -> Self {
    Self::new(attributes.feature_count())
  }
}

impl From<Vec<DeviceFeature>> for ClientDeviceMessageAttributesV2 {
  fn from(value: Vec<DeviceFeature>) -> Self {
    ClientDeviceMessageAttributesV3::from(value).into()
  }
}
