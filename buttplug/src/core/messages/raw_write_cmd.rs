// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
use crate::device::Endpoint;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct RawWriteCmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  pub(super) id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  pub device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Endpoint"))]
  pub endpoint: Endpoint,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Data"))]
  pub data: Vec<u8>,
  #[cfg_attr(feature = "serialize-json", serde(rename = "WriteWithResponse"))]
  pub write_with_response: bool,
}

impl RawWriteCmd {
  pub fn new(
    device_index: u32,
    endpoint: Endpoint,
    data: Vec<u8>,
    write_with_response: bool,
  ) -> Self {
    Self {
      id: 1,
      device_index,
      endpoint,
      data,
      write_with_response,
    }
  }
}
