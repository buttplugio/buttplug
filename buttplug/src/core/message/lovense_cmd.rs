// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
use getset::Getters;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

/// Lovense specific commands (Version 0 Message, **Deprecated**)
// As this message is considered deprecated and is not actually implemented for
// Lovense devices even on spec v1 connections, we can put a null validator on
// it.
#[derive(Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone, Getters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct LovenseCmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Command"))]
  #[getset(get = "pub")]
  command: String,
}

impl LovenseCmd {
  pub fn new(device_index: u32, command: &str) -> Self {
    Self {
      id: 1,
      device_index,
      command: command.to_owned(),
    }
  }
}

impl ButtplugMessageValidator for LovenseCmd {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
