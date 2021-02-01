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
pub struct RSSILevelReading {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "RSSILevel"))]
  rssi_level: i32,
}

impl RSSILevelReading {
  pub fn new(device_index: u32, rssi_level: i32) -> Self {
    Self {
      id: 1,
      device_index,
      rssi_level,
    }
  }

  pub fn rssi_level(&self) -> i32 {
    self.rssi_level
  }
}

impl ButtplugMessageValidator for RSSILevelReading {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    if self.rssi_level > 0 {
      Err(ButtplugMessageError::InvalidMessageContents(format!("RSSI level {} is invalid. RSSI Levels are always negative.", self.rssi_level)))
    } else {
      Ok(())
    }
  }
}