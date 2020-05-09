// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct RotationSubcommand {
  #[cfg_attr(feature = "serialize_json", serde(rename = "Index"))]
  pub index: u32,
  #[cfg_attr(feature = "serialize_json", serde(rename = "Speed"))]
  pub speed: f64,
  #[cfg_attr(feature = "serialize_json", serde(rename = "Clockwise"))]
  pub clockwise: bool,
}

impl RotationSubcommand {
  pub fn new(index: u32, speed: f64, clockwise: bool) -> Self {
    Self {
      index,
      speed,
      clockwise,
    }
  }
}

#[derive(Debug, Default, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct RotateCmd {
  #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
  pub(super) id: u32,
  #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
  pub device_index: u32,
  #[cfg_attr(feature = "serialize_json", serde(rename = "Rotations"))]
  pub rotations: Vec<RotationSubcommand>,
}

impl RotateCmd {
  pub fn new(device_index: u32, rotations: Vec<RotationSubcommand>) -> Self {
    Self {
      id: 1,
      device_index,
      rotations,
    }
  }
}
