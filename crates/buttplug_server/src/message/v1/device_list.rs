// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::device_message_info::DeviceMessageInfoV1;
use crate::message::{
  v0::{DeviceListV0, DeviceMessageInfoV0},
  v2::DeviceListV2,
};
use buttplug_core::{
  errors::ButtplugMessageError,
  message::{ButtplugMessage, ButtplugMessageFinalizer, ButtplugMessageValidator},
};
use getset::Getters;
use serde::{Deserialize, Serialize};

#[derive(
  Default, Clone, Debug, PartialEq, Eq, ButtplugMessage, Getters, Serialize, Deserialize,
)]
pub struct DeviceListV1 {
  #[serde(rename = "Id")]
  pub(in crate::message) id: u32,
  #[serde(rename = "Devices")]
  #[getset(get = "pub")]
  pub(in crate::message) devices: Vec<DeviceMessageInfoV1>,
}

impl From<DeviceListV1> for DeviceListV0 {
  fn from(msg: DeviceListV1) -> Self {
    let mut devices = vec![];
    for d in msg.devices() {
      let dmiv1 = d.clone();
      devices.push(DeviceMessageInfoV0::from(dmiv1));
    }
    Self {
      id: msg.id(),
      devices,
    }
  }
}

impl From<DeviceListV2> for DeviceListV1 {
  fn from(msg: DeviceListV2) -> Self {
    let mut devices = vec![];
    for d in msg.devices() {
      let dmiv2 = d.clone();
      devices.push(DeviceMessageInfoV1::from(dmiv2));
    }
    Self {
      id: msg.id(),
      devices,
    }
  }
}

impl ButtplugMessageValidator for DeviceListV1 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}

impl ButtplugMessageFinalizer for DeviceListV1 {
}
