// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

use super::spec_enums::ButtplugDeviceMessageNameV0;

#[derive(Clone, Debug, PartialEq, Eq, Getters, CopyGetters, Serialize, Deserialize)]
pub struct DeviceMessageInfoV0 {
  #[serde(rename = "DeviceIndex")]
  #[getset(get_copy = "pub")]
  pub(in crate::message) device_index: u32,
  #[serde(rename = "DeviceName")]
  #[getset(get = "pub")]
  pub(in crate::message) device_name: String,
  #[serde(rename = "DeviceMessages")]
  #[getset(get = "pub")]
  pub(in crate::message) device_messages: Vec<ButtplugDeviceMessageNameV0>,
}
