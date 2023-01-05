// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(
  Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone, Getters, CopyGetters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct RawWriteCmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Endpoint"))]
  #[getset(get_copy = "pub")]
  endpoint: Endpoint,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Data"))]
  #[getset(get = "pub")]
  data: Vec<u8>,
  #[cfg_attr(feature = "serialize-json", serde(rename = "WriteWithResponse"))]
  #[getset(get_copy = "pub")]
  write_with_response: bool,
}

impl RawWriteCmd {
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

impl ButtplugMessageValidator for RawWriteCmd {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
