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
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
  InputType,
};
use getset::CopyGetters;
use serde::{Deserialize, Serialize};

#[derive(Debug, Display, PartialEq, Eq, Clone, Serialize, Deserialize, Hash, Copy)]
pub enum InputCommandType {
  Read,
  Subscribe,
  Unsubscribe,
}

#[derive(
  Debug,
  ButtplugDeviceMessage,
  ButtplugMessageFinalizer,
  PartialEq,
  Eq,
  Clone,
  Copy,
  CopyGetters,
  Serialize,
  Deserialize,
)]
pub struct InputCmdV4 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[getset(get_copy = "pub")]
  #[serde(rename = "FeatureIndex")]
  feature_index: u32,
  #[getset(get_copy = "pub")]
  #[serde(rename = "InputType")]
  input_type: InputType,
  #[getset(get_copy = "pub")]
  #[serde(rename = "InputCommand")]
  input_command: InputCommandType,
}

impl InputCmdV4 {
  pub fn new(
    device_index: u32,
    feature_index: u32,
    input_type: InputType,
    input_command_type: InputCommandType,
  ) -> Self {
    Self {
      id: 1,
      device_index,
      feature_index,
      input_type,
      input_command: input_command_type,
    }
  }
}

impl ButtplugMessageValidator for InputCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
    // TODO Should expected_length always be > 0?
  }
}
