// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, ButtplugMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct StopAllDevices {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
}

impl Default for StopAllDevices {
  fn default() -> Self {
    Self { id: 1 }
  }
}

impl ButtplugMessageValidator for StopAllDevices {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
