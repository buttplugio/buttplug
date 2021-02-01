// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct FleshlightLaunchFW12Cmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Position"))]
  position: u8,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Speed"))]
  speed: u8,
}

impl FleshlightLaunchFW12Cmd {
  pub fn new(device_index: u32, position: u8, speed: u8) -> Self {
    Self {
      id: 1,
      device_index,
      position,
      speed,
    }
  }

  pub fn position(&self) -> u8 {
    self.position
  }

  pub fn speed(&self) -> u8 {
    self.speed
  }
}

impl ButtplugMessageValidator for FleshlightLaunchFW12Cmd {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    if !(0..99).contains(&self.speed) {
      Err(ButtplugMessageError::InvalidMessageContents(format!(
        "FleshlightFW12Cmd speed {} invalid, should be between 0 and 99",
        self.speed
      )))
    } else if !(0..99).contains(&self.position) {
      Err(ButtplugMessageError::InvalidMessageContents(format!(
        "FleshlightFW12Cmd position {} invalid, should be between 0 and 99",
        self.position
      )))
    } else {
      Ok(())
    }
  }
}
