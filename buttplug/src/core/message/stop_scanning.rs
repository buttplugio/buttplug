// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, ButtplugMessage, ButtplugMessageFinalizer, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct StopScanning {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
}

impl Default for StopScanning {
  fn default() -> Self {
    Self { id: 1 }
  }
}

impl ButtplugMessageValidator for StopScanning {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
