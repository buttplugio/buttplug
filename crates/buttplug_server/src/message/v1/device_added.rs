// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::v0::{DeviceAddedV0, DeviceMessageInfoV0};
use buttplug_core::{
  errors::ButtplugMessageError,
  message::{ButtplugMessage, ButtplugMessageFinalizer, ButtplugMessageValidator},
};

use super::{device_message_info::DeviceMessageInfoV1, ClientDeviceMessageAttributesV1};

use getset::{CopyGetters, Getters};

use serde::{Deserialize, Serialize};

#[derive(
  ButtplugMessage, Clone, Debug, PartialEq, Eq, Getters, CopyGetters, Serialize, Deserialize,
)]
pub struct DeviceAddedV1 {
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
  pub(in crate::message) device_messages: ClientDeviceMessageAttributesV1,
}

impl From<DeviceAddedV1> for DeviceAddedV0 {
  fn from(msg: DeviceAddedV1) -> Self {
    let id = msg.id();
    let dmiv1 = DeviceMessageInfoV1::from(msg);
    let dmiv0 = DeviceMessageInfoV0::from(dmiv1);

    Self {
      id,
      device_index: dmiv0.device_index(),
      device_name: dmiv0.device_name().clone(),
      device_messages: dmiv0.device_messages().clone(),
    }
  }
}

impl ButtplugMessageValidator for DeviceAddedV1 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_system_id(self.id)
  }
}

impl ButtplugMessageFinalizer for DeviceAddedV1 {
}
