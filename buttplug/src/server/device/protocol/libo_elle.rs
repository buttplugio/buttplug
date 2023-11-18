// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(LiboElle, "libo-elle");

#[derive(Default)]
pub struct LiboElle {}

impl ProtocolHandler for LiboElle {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![{
      let speed = scalar as u8;
      if index == 1 {
        let mut data = 0u8;
        if speed as u8 > 0 && speed <= 7 {
          data |= (speed - 1) << 4;
          data |= 1; // Set the mode too
        } else if speed > 7 {
          data |= (speed - 8) << 4;
          data |= 4; // Set the mode too
        }
        HardwareWriteCmd::new(Endpoint::Tx, vec![data], false).into()
      } else {
        HardwareWriteCmd::new(Endpoint::TxMode, vec![speed], false).into()
      }
    }])
  }
}
