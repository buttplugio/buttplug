// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::message::v1::{ClientDeviceMessageAttributesV1, DeviceMessageInfoV1};

use super::*;
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Getters, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceMessageInfoV2 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  #[getset(get_copy = "pub")]
  pub(in crate::server::message) device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceName"))]
  #[getset(get = "pub")]
  pub(in crate::server::message) device_name: String,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceMessages"))]
  #[getset(get = "pub")]
  pub(in crate::server::message) device_messages: ClientDeviceMessageAttributesV2,
}

impl From<DeviceMessageInfoV2> for DeviceMessageInfoV1 {
  fn from(device_message_info: DeviceMessageInfoV2) -> Self {
    // No structural difference, it's all content changes
    Self {
      device_index: device_message_info.device_index(),
      device_name: device_message_info.device_name().clone(),
      device_messages: ClientDeviceMessageAttributesV1::from(
        device_message_info.device_messages().clone(),
      ),
    }
  }
}

impl From<DeviceAddedV2> for DeviceMessageInfoV2 {
  fn from(device_added: DeviceAddedV2) -> Self {
    // No structural difference, it's all content changes
    Self {
      device_index: device_added.device_index(),
      device_name: device_added.device_name().clone(),
      device_messages: device_added.device_messages().clone(),
    }
  }
}
