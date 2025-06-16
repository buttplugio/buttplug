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
use getset::CopyGetters;
use serde::{Deserialize, Serialize};

#[derive(
  Debug,
  ButtplugDeviceMessage,
  ButtplugMessageFinalizer,
  PartialEq,
  Eq,
  Clone,
  CopyGetters,
  Serialize,
  Deserialize,
)]
pub struct RawReadCmdV2 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[serde(rename = "Endpoint")]
  endpoint: Endpoint,
  #[serde(rename = "ExpectedLength")]
  #[getset(get_copy = "pub")]
  expected_length: u32,
  #[serde(rename = "Timeout")]
  #[getset(get_copy = "pub")]
  timeout: u32,
}

impl RawReadCmdV2 {
  pub fn new(device_index: u32, endpoint: Endpoint, expected_length: u32, timeout: u32) -> Self {
    Self {
      id: 1,
      device_index,
      endpoint,
      expected_length,
      timeout,
    }
  }
}

impl RawCmdEndpoint for RawReadCmdV2 {
  fn endpoint(&self) -> Endpoint {
    self.endpoint
  }
}

impl ButtplugMessageValidator for RawReadCmdV2 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
    // TODO Should expected_length always be > 0?
  }
}
