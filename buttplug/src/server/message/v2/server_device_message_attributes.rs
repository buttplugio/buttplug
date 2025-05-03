// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::message::{ActuatorType, SensorType},
  server::message::{
    server_device_feature::ServerDeviceFeature,
    v1::NullDeviceMessageAttributesV1,
    ServerDeviceMessageAttributesV3,
    ServerGenericDeviceMessageAttributesV3,
  },
};
use getset::{CopyGetters, Getters, Setters};
use serde::{Deserialize, Serialize};

use super::RawDeviceMessageAttributesV2;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct ServerDeviceMessageAttributesV2 {
  // Generic commands
  #[getset(get = "pub")]
  #[serde(rename = "VibrateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) vibrate_cmd: Option<ServerGenericDeviceMessageAttributesV2>,
  #[getset(get = "pub")]
  #[serde(rename = "RotateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) rotate_cmd: Option<ServerGenericDeviceMessageAttributesV2>,
  #[getset(get = "pub")]
  #[serde(rename = "LinearCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) linear_cmd: Option<ServerGenericDeviceMessageAttributesV2>,
  #[getset(get = "pub")]
  #[serde(rename = "BatteryLevelCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) battery_level_cmd: Option<ServerSensorDeviceMessageAttributesV2>,

  // RSSILevel is added post-serialization (only for bluetooth devices)
  #[getset(get = "pub")]
  #[serde(rename = "RSSILevelCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) rssi_level_cmd: Option<ServerSensorDeviceMessageAttributesV2>,

  // StopDeviceCmd always exists
  #[getset(get = "pub")]
  #[serde(rename = "StopDeviceCmd")]
  pub(in crate::server::message) stop_device_cmd: NullDeviceMessageAttributesV1,

  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(rename = "RawReadCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) raw_read_cmd: Option<RawDeviceMessageAttributesV2>,
  #[getset(get = "pub")]
  #[serde(rename = "RawWriteCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) raw_write_cmd: Option<RawDeviceMessageAttributesV2>,
  #[getset(get = "pub")]
  #[serde(rename = "RawSubscribeCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) raw_subscribe_cmd: Option<RawDeviceMessageAttributesV2>,
  #[getset(get = "pub")]
  #[serde(rename = "RawUnsubscribeCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) raw_unsubscribe_cmd: Option<RawDeviceMessageAttributesV2>,

  // Needed to load from config for fallback, but unused here.
  #[getset(get = "pub")]
  #[serde(rename = "FleshlightLaunchFW12Cmd")]
  #[serde(skip)]
  pub(in crate::server::message) fleshlight_launch_fw12_cmd: Option<NullDeviceMessageAttributesV1>,
  #[getset(get = "pub")]
  #[serde(rename = "VorzeA10CycloneCmd")]
  #[serde(skip)]
  pub(in crate::server::message) vorze_a10_cyclone_cmd: Option<NullDeviceMessageAttributesV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, CopyGetters, Setters)]
pub struct ServerGenericDeviceMessageAttributesV2 {
  #[getset(get_copy = "pub")]
  #[serde(rename = "FeatureCount")]
  pub(in crate::server::message) feature_count: u32,
  #[getset(get = "pub")]
  #[serde(rename = "StepCount")]
  pub(in crate::server::message) step_count: Vec<u32>,
  #[getset(get = "pub")]
  #[serde(skip)]
  pub(in crate::server::message) features: Vec<ServerDeviceFeature>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, Getters, Setters)]
pub struct ServerSensorDeviceMessageAttributesV2 {
  #[getset(get = "pub")]
  #[serde(skip)]
  feature: ServerDeviceFeature,
}

impl ServerSensorDeviceMessageAttributesV2 {
  pub fn new(feature: &ServerDeviceFeature) -> Self {
    Self {
      feature: feature.clone(),
    }
  }
}

impl From<Vec<ServerDeviceFeature>> for ServerDeviceMessageAttributesV2 {
  fn from(value: Vec<ServerDeviceFeature>) -> Self {
    ServerDeviceMessageAttributesV3::from(value).into()
  }
}

pub fn vibrate_cmd_from_scalar_cmd(
  attributes_vec: &[ServerGenericDeviceMessageAttributesV3],
) -> ServerGenericDeviceMessageAttributesV2 {
  let mut feature_count = 0u32;
  let mut step_count = vec![];
  let mut features = vec![];
  for attr in attributes_vec {
    if *attr.actuator_type() == ActuatorType::Vibrate {
      feature_count += 1;
      step_count.push(*attr.step_count());
      features.push(attr.feature().clone());
    }
  }
  ServerGenericDeviceMessageAttributesV2 {
    feature_count,
    step_count,
    features,
  }
}

impl From<ServerDeviceMessageAttributesV3> for ServerDeviceMessageAttributesV2 {
  fn from(other: ServerDeviceMessageAttributesV3) -> Self {
    Self {
      vibrate_cmd: other
        .scalar_cmd()
        .as_ref()
        .map(|x| vibrate_cmd_from_scalar_cmd(x))
        .filter(|x| x.feature_count() != 0),
      rotate_cmd: other
        .rotate_cmd()
        .as_ref()
        .map(|x| ServerGenericDeviceMessageAttributesV2::from(x.clone())),
      linear_cmd: other
        .linear_cmd()
        .as_ref()
        .map(|x| ServerGenericDeviceMessageAttributesV2::from(x.clone())),
      battery_level_cmd: {
        if let Some(sensor_info) = other.sensor_read_cmd() {
          sensor_info
            .iter()
            .find(|x| *x.sensor_type() == SensorType::Battery)
            .map(|attr| ServerSensorDeviceMessageAttributesV2::new(attr.feature()))
        } else {
          None
        }
      },
      rssi_level_cmd: {
        if let Some(sensor_info) = other.sensor_read_cmd() {
          sensor_info
            .iter()
            .find(|x| *x.sensor_type() == SensorType::RSSI)
            .map(|attr| ServerSensorDeviceMessageAttributesV2::new(attr.feature()))
        } else {
          None
        }
      },
      stop_device_cmd: other.stop_device_cmd().clone(),
      raw_read_cmd: other.raw_read_cmd().clone(),
      raw_write_cmd: other.raw_write_cmd().clone(),
      raw_subscribe_cmd: other.raw_subscribe_cmd().clone(),
      raw_unsubscribe_cmd: other.raw_subscribe_cmd().clone(),
      fleshlight_launch_fw12_cmd: other.fleshlight_launch_fw12_cmd().clone(),
      vorze_a10_cyclone_cmd: other.vorze_a10_cyclone_cmd().clone(),
    }
  }
}

impl From<Vec<ServerGenericDeviceMessageAttributesV3>> for ServerGenericDeviceMessageAttributesV2 {
  fn from(attributes_vec: Vec<ServerGenericDeviceMessageAttributesV3>) -> Self {
    Self {
      feature_count: attributes_vec.len() as u32,
      step_count: attributes_vec.iter().map(|x| *x.step_count()).collect(),
      features: attributes_vec.iter().map(|x| x.feature().clone()).collect(),
    }
  }
}
