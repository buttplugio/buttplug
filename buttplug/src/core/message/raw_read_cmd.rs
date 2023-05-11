// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
use getset::CopyGetters;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(
  Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone, CopyGetters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct RawReadCmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Endpoint"))]
  #[getset(get_copy = "pub")]
  endpoint: Endpoint,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ExpectedLength"))]
  #[getset(get_copy = "pub")]
  expected_length: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Timeout"))]
  #[getset(get_copy = "pub")]
  timeout: u32,
}

impl RawReadCmd {
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

impl ButtplugMessageValidator for RawReadCmd {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
    // TODO Should expected_length always be > 0?
  }
}
