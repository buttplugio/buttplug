// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{core::message::DeviceMessageInfoV4, server::message::v2::DeviceMessageInfoV2};

use super::*;
use getset::{CopyGetters, Getters, MutGetters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

/// Substructure of device messages, used for attribute information (name, messages supported, etc...)
#[derive(Clone, Debug, PartialEq, MutGetters, Getters, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceMessageInfoV3 {
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
    serde(
      rename = "DeviceMessageTimingGap",
      skip_serializing_if = "Option::is_none"
    )
  )]
  #[getset(get = "pub")]
  device_message_timing_gap: Option<u32>,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceMessages"))]
  #[getset(get = "pub", get_mut = "pub(super)")]
  device_messages: ClientDeviceMessageAttributesV3,
}

impl DeviceMessageInfoV3 {
  pub fn new(
    device_index: u32,
    device_name: &str,
    device_display_name: &Option<String>,
    device_message_timing_gap: &Option<u32>,
    device_messages: ClientDeviceMessageAttributesV3,
  ) -> Self {
    Self {
      device_index,
      device_name: device_name.to_owned(),
      device_display_name: device_display_name.clone(),
      device_message_timing_gap: *device_message_timing_gap,
      device_messages,
    }
  }
}

impl From<DeviceAddedV3> for DeviceMessageInfoV3 {
  fn from(device_added: DeviceAddedV3) -> Self {
    Self {
      device_index: device_added.device_index(),
      device_name: device_added.device_name().clone(),
      device_display_name: device_added.device_display_name().clone(),
      device_message_timing_gap: *device_added.device_message_timing_gap(),
      device_messages: device_added.device_messages().clone(),
    }
  }
}

impl From<DeviceMessageInfoV3> for DeviceMessageInfoV2 {
  fn from(device_message_info: DeviceMessageInfoV3) -> Self {
    // No structural difference, it's all content changes
    Self {
      device_index: device_message_info.device_index(),
      device_name: device_message_info.device_name().clone(),
      device_messages: device_message_info.device_messages().clone().into(),
    }
  }
}

impl From<DeviceMessageInfoV4> for DeviceMessageInfoV3 {
  fn from(value: DeviceMessageInfoV4) -> Self {
    DeviceMessageInfoV3::new(
      value.device_index(),
      value.device_name(),
      value.device_display_name(),
      &None,
      value.device_features().clone().into(),
    )
  }
}
