// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::DeviceAddedV4;
use crate::core::message::DeviceFeature;
use getset::{CopyGetters, Getters, MutGetters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

/// Substructure of device messages, used for attribute information (name, messages supported, etc...)
#[derive(Clone, Debug, PartialEq, MutGetters, Getters, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceMessageInfoV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  #[getset(get_copy = "pub")]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceName"))]
  #[getset(get = "pub")]
  device_name: String,
  #[cfg_attr(
    feature = "serialize-json",
    serde(rename = "DeviceDisplayName", skip_serializing_if = "Option::is_none")
  )]
  #[getset(get = "pub")]
  device_display_name: Option<String>,
  #[cfg_attr(
    feature = "serialize-json",
    serde(rename = "DeviceMessageTimingGap")
  )]
  #[getset(get_copy = "pub")]
  device_message_timing_gap: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceFeatures"))]
  #[getset(get = "pub", get_mut = "pub(super)")]
  device_features: Vec<DeviceFeature>,
}

impl DeviceMessageInfoV4 {
  pub fn new(
    device_index: u32,
    device_name: &str,
    device_display_name: &Option<String>,
    device_message_timing_gap: u32,
    device_features: &Vec<DeviceFeature>,
  ) -> Self {
    Self {
      device_index,
      device_name: device_name.to_owned(),
      device_display_name: device_display_name.clone(),
      device_message_timing_gap: device_message_timing_gap,
      device_features: device_features.clone(),
    }
  }
}

impl From<DeviceAddedV4> for DeviceMessageInfoV4 {
  fn from(device_added: DeviceAddedV4) -> Self {
    Self {
      device_index: device_added.device_index(),
      device_name: device_added.device_name().clone(),
      device_display_name: device_added.device_display_name().clone(),
      device_message_timing_gap: device_added.device_message_timing_gap(),
      device_features: device_added.device_features().clone(),
    }
  }
}
