// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct RawWriteCmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Endpoint"))]
  endpoint: Endpoint,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Data"))]
  data: Vec<u8>,
  #[cfg_attr(feature = "serialize-json", serde(rename = "WriteWithResponse"))]
  write_with_response: bool,
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

  pub fn endpoint(&self) -> Endpoint {
    self.endpoint
  }

  pub fn data(&self) -> &Vec<u8> {
    &self.data
  }

  pub fn write_with_response(&self) -> bool {
    self.write_with_response
  }
}

impl ButtplugMessageValidator for RawWriteCmd {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
