// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  ButtplugDeviceMessage,
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
pub struct StopDeviceCmdV4 {
  id: u32,
  device_index: u32,
  #[serde(default = "mk_true")]
  #[getset(get_copy = "pub")]
  inputs: bool,
  #[getset(get_copy = "pub")]
  #[serde(default = "mk_true")]
  outputs: bool,
}

impl StopDeviceCmdV4 {
  pub fn new(device_index: u32, inputs: bool, outputs: bool) -> Self {
    Self {
      id: 1,
      device_index,
      inputs,
      outputs,
    }
  }
}

impl ButtplugMessage for StopDeviceCmdV4 {
  fn id(&self) -> u32 {
    self.id
  }
  fn set_id(&mut self, id: u32) {
    self.id = id;
  }
}

impl ButtplugDeviceMessage for StopDeviceCmdV4 {
  fn device_index(&self) -> u32 {
    self.device_index
  }
  fn set_device_index(&mut self, device_index: u32) {
    self.device_index = device_index;
  }
}

impl ButtplugMessageValidator for StopDeviceCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
