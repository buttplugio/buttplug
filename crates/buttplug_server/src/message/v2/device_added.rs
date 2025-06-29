// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{device_message_info::DeviceMessageInfoV2, ClientDeviceMessageAttributesV2};
use crate::message::v1::{DeviceAddedV1, DeviceMessageInfoV1};
use buttplug_core::{
  errors::ButtplugMessageError,
  message::{ButtplugMessage, ButtplugMessageFinalizer, ButtplugMessageValidator},
};

use getset::{CopyGetters, Getters};

use serde::{Deserialize, Serialize};

#[derive(
  ButtplugMessage, Clone, Debug, PartialEq, Eq, Getters, CopyGetters, Serialize, Deserialize,
)]
pub struct DeviceAddedV2 {
  #[serde(rename = "Id")]
  pub(in crate::message) id: u32,
  #[serde(rename = "DeviceIndex")]
  #[getset(get_copy = "pub")]
  pub(in crate::message) device_index: u32,
  #[serde(rename = "DeviceName")]
  #[getset(get = "pub")]
  pub(in crate::message) device_name: String,
  #[serde(rename = "DeviceMessages")]
  #[getset(get = "pub")]
  pub(in crate::message) device_messages: ClientDeviceMessageAttributesV2,
}

impl From<DeviceAddedV2> for DeviceAddedV1 {
  fn from(msg: DeviceAddedV2) -> Self {
    let id = msg.id();
    let dmiv2 = DeviceMessageInfoV2::from(msg);
    let dmiv1 = DeviceMessageInfoV1::from(dmiv2);

    Self {
      id,
      device_index: dmiv1.device_index(),
      device_name: dmiv1.device_name().clone(),
      device_messages: dmiv1.device_messages().clone(),
    }
  }
}

impl ButtplugMessageValidator for DeviceAddedV2 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_system_id(self.id)
  }
}

impl ButtplugMessageFinalizer for DeviceAddedV2 {
}
