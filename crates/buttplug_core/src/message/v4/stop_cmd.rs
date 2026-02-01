// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  ButtplugMessage,
  ButtplugMessageError,
  ButtplugMessageValidator,
};
use getset::CopyGetters;
use serde::{Deserialize, Serialize};

fn mk_true() -> bool {
  true
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, CopyGetters)]
#[serde(rename_all = "PascalCase")]
pub struct StopCmdV4 {
  id: u32,
  #[getset(get_copy = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  device_index: Option<u32>,
  #[getset(get_copy = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  feature_index: Option<u32>,
  #[serde(default = "mk_true")]
  #[getset(get_copy = "pub")]
  inputs: bool,
  #[getset(get_copy = "pub")]
  #[serde(default = "mk_true")]
  outputs: bool,
}

impl StopCmdV4 {
  pub fn new(device_index: Option<u32>, feature_index: Option<u32>, inputs: bool, outputs: bool) -> Self {
    Self {
      id: 1,
      device_index,
      feature_index,
      inputs,
      outputs,
    }
  }
}

/// Implementation for StopCmdV4 default trait. Just works as StopAllDevices did, stopping all traits across all devices.
impl Default for StopCmdV4 {
  fn default() -> Self {
    Self {
      id: 1,
      device_index: None,
      feature_index: None,
      inputs: true,
      outputs: true
    }
  }
}

impl ButtplugMessage for StopCmdV4 {
  fn id(&self) -> u32 {
    self.id
  }
  fn set_id(&mut self, id: u32) {
    self.id = id;
  }
}

impl ButtplugMessageValidator for StopCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
