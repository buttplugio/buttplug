// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::{
  ButtplugDeviceMessage,
  ButtplugMessage,
  ButtplugMessageError,
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
};
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

/// Move device to a certain position in a certain amount of time
#[derive(Debug, PartialEq, Clone, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
#[getset(get_copy = "pub")]
pub struct VectorSubcommandV1 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Duration"))]
  duration: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Position"))]
  position: f64,
}

impl VectorSubcommandV1 {
  pub fn new(index: u32, duration: u32, position: f64) -> Self {
    Self {
      index,
      duration,
      position,
    }
  }
}

#[derive(Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, Getters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct LinearCmdV1 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Vectors"))]
  #[getset(get = "pub")]
  vectors: Vec<VectorSubcommandV1>,
}

impl LinearCmdV1 {
  pub fn new(device_index: u32, vectors: Vec<VectorSubcommandV1>) -> Self {
    Self {
      id: 1,
      device_index,
      vectors,
    }
  }
}

impl ButtplugMessageValidator for LinearCmdV1 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    for vec in &self.vectors {
      self.is_in_command_range(
        vec.position,
        format!(
          "VectorSubcommand position {} for index {} is invalid, should be between 0.0 and 1.0",
          vec.position, vec.index
        ),
      )?;
    }
    Ok(())
  }
}
