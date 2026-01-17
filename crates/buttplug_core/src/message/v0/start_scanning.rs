// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{ButtplugMessage, ButtplugMessageError, ButtplugMessageValidator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartScanningV0 {
  #[serde(rename = "Id")]
  id: u32,
}

impl Default for StartScanningV0 {
  fn default() -> Self {
    Self { id: 1 }
  }
}

impl ButtplugMessage for StartScanningV0 {
  fn id(&self) -> u32 {
    self.id
  }
  fn set_id(&mut self, id: u32) {
    self.id = id;
  }
}

impl ButtplugMessageValidator for StartScanningV0 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
