// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct VectorSubcommand {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  pub index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Duration"))]
  pub duration: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Position"))]
  pub position: f64,
}

impl VectorSubcommand {
  pub fn new(index: u32, duration: u32, position: f64) -> Self {
    Self {
      index,
      duration,
      position,
    }
  }

  pub fn index(&self) -> u32 {
    self.index
  }

  pub fn duration(&self) -> u32 {
    self.duration
  }

  pub fn position(&self) -> &f64 {
    &self.position
  }
}

#[derive(Debug, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct LinearCmd {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Vectors"))]
  vectors: Vec<VectorSubcommand>,
}

impl LinearCmd {
  pub fn new(device_index: u32, vectors: Vec<VectorSubcommand>) -> Self {
    Self {
      id: 1,
      device_index,
      vectors,
    }
  }

  pub fn vectors(&self) -> &Vec<VectorSubcommand> {
    &self.vectors
  }
}

impl ButtplugMessageValidator for LinearCmd {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    for vec in &self.vectors {
      self.is_in_command_range(vec.position, format!("VectorSubcommand position {} for index {} is invalid, should be between 0.0 and 1.0", vec.position, vec.index))?;
    }
    Ok(())
  }
}