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
pub struct RawSubscribeCmdV2 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[serde(rename = "Endpoint")]
  endpoint: Endpoint,
}

impl RawSubscribeCmdV2 {
  pub fn new(device_index: u32, endpoint: Endpoint) -> Self {
    Self {
      id: 1,
      device_index,
      endpoint,
    }
  }
}

impl RawCmdEndpoint for RawSubscribeCmdV2 {
  fn endpoint(&self) -> Endpoint {
    self.endpoint
  }
}

impl ButtplugMessageValidator for RawSubscribeCmdV2 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
