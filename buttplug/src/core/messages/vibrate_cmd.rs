// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct VibrateSubcommand {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  pub index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Speed"))]
  pub speed: f64,
}

impl VibrateSubcommand {
  pub fn new(index: u32, speed: f64) -> Self {
    Self { index, speed }
  }
}

#[derive(Debug, Default, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct VibrateCmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  pub(super) id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  pub device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Speeds"))]
  pub speeds: Vec<VibrateSubcommand>,
}

impl VibrateCmd {
  pub fn new(device_index: u32, speeds: Vec<VibrateSubcommand>) -> Self {
    Self {
      id: 1,
      device_index,
      speeds,
    }
  }
}
