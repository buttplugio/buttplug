// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{ButtplugMessage, ButtplugMessageError, ButtplugMessageValidator};
use getset::CopyGetters;
use serde::{Deserialize, Serialize};

fn mk_true() -> bool {
  true
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, CopyGetters)]
#[serde(rename_all = "PascalCase")]
pub struct StopAllDevicesV4 {
  id: u32,
  #[serde(default = "mk_true")]
  #[getset(get_copy = "pub")]
  inputs: bool,
  #[serde(default = "mk_true")]
  #[getset(get_copy = "pub")]
  outputs: bool,
}

impl Default for StopAllDevicesV4 {
  fn default() -> Self {
    Self {
      id: 1,
      inputs: true,
      outputs: true,
    }
  }
}

impl ButtplugMessage for StopAllDevicesV4 {
  fn id(&self) -> u32 {
    self.id
  }
  fn set_id(&mut self, id: u32) {
    self.id = id;
  }
}

impl ButtplugMessageValidator for StopAllDevicesV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
