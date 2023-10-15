// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

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

generic_protocol_setup!(Motorbunny, "motorbunny");

#[derive(Default)]
pub struct Motorbunny {}

impl ProtocolHandler for Motorbunny {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut command_vec: Vec<u8>;
    if scalar == 0 {
      command_vec = vec![0xf0, 0x00, 0x00, 0x00, 0x00, 0xec];
    } else {
      command_vec = vec![0xff];
      let mut vibe_commands = [scalar as u8, 0x14].repeat(7);
      let crc = vibe_commands
        .iter()
        .fold(0u8, |a, b| a.overflowing_add(*b).0);
      command_vec.append(&mut vibe_commands);
      command_vec.append(&mut vec![crc, 0xec]);
    }
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      command_vec,
      false,
    )
    .into()])
  }
}
