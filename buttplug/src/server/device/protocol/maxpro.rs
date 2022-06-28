// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, messages::Endpoint},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(Maxpro, "maxpro");

#[derive(Default)]
pub struct Maxpro {}

impl ProtocolHandler for Maxpro {
  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut data = vec![
      0x55u8,
      0x04,
      0x07,
      0xff,
      0xff,
      0x3f,
      scalar as u8,
      0x5f,
      scalar as u8,
      0x00,
    ];
    let mut crc: u8 = 0;

    for b in data.clone() {
      crc = crc.wrapping_add(b);
    }

    data[9] = crc;
    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, false).into()])
  }
}

// TODO Write some tests! Especially with the weird operational range on this.
