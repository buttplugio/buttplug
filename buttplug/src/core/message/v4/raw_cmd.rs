// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::{
  ButtplugDeviceMessage,
  ButtplugMessage,
  ButtplugMessageError,
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
  Endpoint,
};
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

pub trait RawCmdEndpoint {
  fn endpoint(&self) -> Endpoint;
}

#[derive(Debug, Display, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum RawCommand {
  Read(RawCommandRead),
  Write(RawCommandWrite),
  Subscribe,
  Unsubscribe,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct RawCommandRead {
  #[serde(rename = "ExpectedLength")]
  expected_length: u32,
  #[serde(rename = "Timeout")]
  timeout: u32,
}

impl RawCommandRead {
  pub fn new(expected_length: u32, timeout: u32) -> Self {
    Self {
      expected_length,
      timeout,
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Getters, CopyGetters)]
pub struct RawCommandWrite {
  #[serde(rename = "Data")]
  #[getset(get = "pub")]
  data: Vec<u8>,
  #[serde(rename = "WriteWithResponse")]
  #[getset(get_copy = "pub")]
  write_with_response: bool,
}

impl RawCommandWrite {
  pub fn new(data: &Vec<u8>, write_with_response: bool) -> Self {
    Self {
      data: data.clone(),
      write_with_response,
    }
  }
}

#[derive(
  Debug,
  ButtplugDeviceMessage,
  ButtplugMessageFinalizer,
  PartialEq,
  Eq,
  Clone,
  Getters,
  Serialize,
  Deserialize,
)]
pub struct RawCmdV4 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[serde(rename = "Endpoint")]
  endpoint: Endpoint,
  #[getset(get = "pub")]
  #[serde(rename = "RawCommand")]
  raw_command: RawCommand,
}

impl RawCmdV4 {
  pub fn new(device_index: u32, endpoint: Endpoint, raw_command: RawCommand) -> Self {
    Self {
      id: 1,
      device_index,
      endpoint,
      raw_command,
    }
  }
}

impl ButtplugMessageValidator for RawCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
    // TODO Should expected_length always be > 0?
  }
}

impl RawCmdEndpoint for RawCmdV4 {
  fn endpoint(&self) -> Endpoint {
    self.endpoint
  }
}
