// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::{
  errors::ButtplugMessageError,
  message::{
    ButtplugDeviceMessage,
    ButtplugMessage,
    ButtplugMessageFinalizer,
    ButtplugMessageValidator,
    Endpoint,
    RawCmdEndpoint,
  },
};
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

#[derive(
  Debug,
  ButtplugDeviceMessage,
  ButtplugMessageFinalizer,
  PartialEq,
  Eq,
  Clone,
  Getters,
  CopyGetters,
  Serialize,
  Deserialize,
)]
pub struct RawWriteCmdV2 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[serde(rename = "Endpoint")]
  endpoint: Endpoint,
  #[serde(rename = "Data")]
  #[getset(get = "pub")]
  data: Vec<u8>,
  #[serde(rename = "WriteWithResponse")]
  #[getset(get_copy = "pub")]
  write_with_response: bool,
}

impl RawWriteCmdV2 {
  pub fn new(
    device_index: u32,
    endpoint: Endpoint,
    data: &[u8],
    write_with_response: bool,
  ) -> Self {
    Self {
      id: 1,
      device_index,
      endpoint,
      data: data.to_vec(),
      write_with_response,
    }
  }
}

impl RawCmdEndpoint for RawWriteCmdV2 {
  fn endpoint(&self) -> Endpoint {
    self.endpoint
  }
}

impl ButtplugMessageValidator for RawWriteCmdV2 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
