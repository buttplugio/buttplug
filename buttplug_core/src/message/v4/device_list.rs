// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::DeviceMessageInfoV4;
use crate::message::{
  ButtplugMessage,
  ButtplugMessageError,
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
};
use getset::Getters;
use serde::{Deserialize, Serialize};

/// List of all devices currently connected to the server.
#[derive(Default, Clone, Debug, PartialEq, ButtplugMessage, Getters, Serialize, Deserialize)]
pub struct DeviceListV4 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "Devices")]
  #[getset(get = "pub")]
  devices: Vec<DeviceMessageInfoV4>,
}

impl DeviceListV4 {
  pub fn new(devices: Vec<DeviceMessageInfoV4>) -> Self {
    Self { id: 1, devices }
  }
}

impl ButtplugMessageValidator for DeviceListV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}

impl ButtplugMessageFinalizer for DeviceListV4 {
  fn finalize(&mut self) {
  }
}
