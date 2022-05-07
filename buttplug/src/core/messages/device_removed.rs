// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Notification that a device has disconnected from the server.

use super::*;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, ButtplugMessage, Clone, PartialEq)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceRemoved {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
}

impl DeviceRemoved {
  pub fn new(device_index: u32) -> Self {
    Self {
      id: 0,
      device_index,
    }
  }

  pub fn device_index(&self) -> u32 {
    self.device_index
  }
}

impl ButtplugMessageValidator for DeviceRemoved {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_system_id(self.id)
  }
}
