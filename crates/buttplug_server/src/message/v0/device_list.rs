// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::device_message_info::DeviceMessageInfoV0;
use buttplug_core::{
  errors::ButtplugMessageError,
  message::{ButtplugMessage, ButtplugMessageFinalizer, ButtplugMessageValidator},
};
use getset::Getters;
use serde::{Deserialize, Serialize};

#[derive(
  Default, Clone, Debug, PartialEq, Eq, ButtplugMessage, Getters, Serialize, Deserialize,
)]
pub struct DeviceListV0 {
  #[serde(rename = "Id")]
  pub(in crate::message) id: u32,
  #[serde(rename = "Devices")]
  #[getset(get = "pub")]
  pub(in crate::message) devices: Vec<DeviceMessageInfoV0>,
}

impl ButtplugMessageValidator for DeviceListV0 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}

impl ButtplugMessageFinalizer for DeviceListV0 {
}
