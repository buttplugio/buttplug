// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.


use super::*;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

/// Kiiroo Command (Version 0 Message, Deprecated)
#[deprecated(since="0.0.0", note="Buttplug Spec Version 0 message, no longer used and not supported by any device.")]
#[derive(Debug, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct KiirooCmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Command"))]
  command: String,
}

impl KiirooCmd {
  pub fn new(device_index: u32, command: &str) -> Self {
    Self {
      id: 1,
      device_index,
      command: command.to_owned(),
    }
  }

  pub fn command(&self) -> &String {
    &self.command
  }
}

impl ButtplugMessageValidator for KiirooCmd {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
