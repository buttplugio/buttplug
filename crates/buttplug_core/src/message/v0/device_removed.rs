// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Notification that a device has disconnected from the server.

use crate::message::{
  ButtplugMessage,
  ButtplugMessageError,
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
};
use getset::CopyGetters;
use serde::{Deserialize, Serialize};

#[derive(
  Debug, Default, ButtplugMessage, Clone, PartialEq, Eq, CopyGetters, Serialize, Deserialize,
)]
pub struct DeviceRemovedV0 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  #[getset(get_copy = "pub")]
  device_index: u32,
}

impl DeviceRemovedV0 {
  pub fn new(device_index: u32) -> Self {
    Self {
      id: 0,
      device_index,
    }
  }
}

impl ButtplugMessageValidator for DeviceRemovedV0 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_system_id(self.id)
  }
}

impl ButtplugMessageFinalizer for DeviceRemovedV0 {
}
