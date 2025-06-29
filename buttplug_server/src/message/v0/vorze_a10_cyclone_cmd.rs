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
  },
};
use getset::CopyGetters;
use serde::{Deserialize, Serialize};

#[derive(
  Debug,
  ButtplugDeviceMessage,
  ButtplugMessageFinalizer,
  Default,
  PartialEq,
  Eq,
  Clone,
  CopyGetters,
  Serialize,
  Deserialize,
)]
pub struct VorzeA10CycloneCmdV0 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[serde(rename = "Speed")]
  #[getset(get_copy = "pub")]
  speed: u32,
  #[serde(rename = "Clockwise")]
  #[getset(get_copy = "pub")]
  clockwise: bool,
}

impl VorzeA10CycloneCmdV0 {
  pub fn new(device_index: u32, speed: u32, clockwise: bool) -> Self {
    Self {
      id: 1,
      device_index,
      speed,
      clockwise,
    }
  }
}

impl ButtplugMessageValidator for VorzeA10CycloneCmdV0 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    if self.speed > 99 {
      Err(ButtplugMessageError::InvalidMessageContents(format!(
        "Speed {} for VorzeA10CycloneCmd is invalid. Speed should be a value between 0.0 and 1.0",
        self.speed
      )))
    } else {
      Ok(())
    }
  }
}
