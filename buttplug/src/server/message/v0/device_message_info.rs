// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

use crate::server::message::ButtplugDeviceMessageType;

#[derive(Clone, Debug, PartialEq, Eq, Getters, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceMessageInfoV0 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  #[getset(get_copy = "pub")]
  pub(in crate::server::message) device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceName"))]
  #[getset(get = "pub")]
  pub(in crate::server::message) device_name: String,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceMessages"))]
  #[getset(get = "pub")]
  pub(in crate::server::message) device_messages: Vec<ButtplugDeviceMessageType>,
}
