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

generic_protocol_setup!(ManNuo, "mannuo");

#[derive(Default)]
pub struct ManNuo {}

impl ProtocolHandler for ManNuo {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut data = vec![0xAA, 0x55, 0x06, 0x01, 0x01, 0x01, scalar as u8, 0xFA];
    // Simple XOR of everything up to the 9th byte for CRC.
    let mut crc: u8 = 0;
    for b in data.clone() {
      crc ^= b;
    }
    data.push(crc);
    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, true).into()])
  }
}
