// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct FleshlightLaunchFW12Cmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  pub(super) id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  pub device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Position"))]
  pub position: u8,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Speed"))]
  pub speed: u8,
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
}
