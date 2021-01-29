// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, ButtplugDeviceMessage, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct VorzeA10CycloneCmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  pub(super) id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  pub device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Speed"))]
  pub speed: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Clockwise"))]
  pub clockwise: bool,
}

impl VorzeA10CycloneCmd {
  pub fn new(device_index: u32, speed: u32, clockwise: bool) -> Self {
    Self {
      id: 1,
      device_index,
      speed,
      clockwise,
    }
  }
}

impl ButtplugMessageValidator for VorzeA10CycloneCmd {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    if self.speed > 99 {
      Err(ButtplugMessageError::InvalidMessageContents(format!("Speed {} for VorzeA10CycloneCmd is invalid. Speed should be a value between 0.0 and 1.0", self.speed)))
    } else {
      Ok(())
    }
  }
}