// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Fleshlight FW v1.2 Command (Version 0 Message, Deprecated)

use crate::core::{errors::ButtplugMessageError, message::{
  ButtplugDeviceMessage,
  ButtplugMessage,
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
}};
use getset::CopyGetters;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(
  Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone, CopyGetters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct FleshlightLaunchFW12CmdV0 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Position"))]
  #[getset(get_copy = "pub")]
  position: u8,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Speed"))]
  #[getset(get_copy = "pub")]
  speed: u8,
}

impl FleshlightLaunchFW12CmdV0 {
  pub fn new(device_index: u32, position: u8, speed: u8) -> Self {
    Self {
      id: 1,
      device_index,
      position,
      speed,
    }
  }
}

impl ButtplugMessageValidator for FleshlightLaunchFW12CmdV0 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    if !(0..100).contains(&self.speed) {
      Err(ButtplugMessageError::InvalidMessageContents(format!(
        "FleshlightFW12Cmd speed {} invalid, should be between 0 and 99",
        self.speed
      )))
    } else if !(0..100).contains(&self.position) {
      Err(ButtplugMessageError::InvalidMessageContents(format!(
        "FleshlightFW12Cmd position {} invalid, should be between 0 and 99",
        self.position
      )))
    } else {
      Ok(())
    }
  }
}

#[cfg(test)]
mod test {
  use super::{ButtplugMessageValidator, FleshlightLaunchFW12CmdV0};

  #[test]
  pub fn test_legacy_fleshlight_message_bounds() {
    assert!(FleshlightLaunchFW12CmdV0::new(0, 0, 0).is_valid().is_ok());
    assert!(FleshlightLaunchFW12CmdV0::new(0, 99, 99).is_valid().is_ok());
    assert!(FleshlightLaunchFW12CmdV0::new(0, 100, 99)
      .is_valid()
      .is_err());
    assert!(FleshlightLaunchFW12CmdV0::new(0, 99, 100)
      .is_valid()
      .is_err());
  }
}
