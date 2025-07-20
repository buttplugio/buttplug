// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::collections::BTreeMap;

use crate::message::DeviceFeature;
use getset::{CopyGetters, Getters, MutGetters};
use serde::{Deserialize, Serialize};

/// Substructure of device messages, used for attribute information (name, messages supported, etc...)
#[derive(Clone, Debug, PartialEq, MutGetters, Getters, CopyGetters, Serialize, Deserialize)]
pub struct DeviceMessageInfoV4 {
  #[serde(rename = "DeviceIndex")]
  #[getset(get_copy = "pub")]
  device_index: u32,
  #[serde(rename = "DeviceName")]
  #[getset(get = "pub")]
  device_name: String,
  #[serde(rename = "DeviceDisplayName", skip_serializing_if = "Option::is_none")]
  #[getset(get = "pub")]
  device_display_name: Option<String>,
  #[serde(rename = "DeviceMessageTimingGap")]
  #[getset(get_copy = "pub")]
  device_message_timing_gap: u32,
  #[serde(rename = "DeviceFeatures")]
  #[getset(get = "pub", get_mut = "pub(super)")]
  device_features: BTreeMap<u32, DeviceFeature>,
}

impl DeviceMessageInfoV4 {
  pub fn new(
    device_index: u32,
    device_name: &str,
    device_display_name: &Option<String>,
    device_message_timing_gap: u32,
    device_features: &Vec<DeviceFeature>,
  ) -> Self {
    let feature_map = device_features.iter().map(|x| (x.feature_index(), x.clone())).collect();
    Self {
      device_index,
      device_name: device_name.to_owned(),
      device_display_name: device_display_name.clone(),
      device_message_timing_gap,
      device_features: feature_map,
    }
  }
}
