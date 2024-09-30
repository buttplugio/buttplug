// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  generic_protocol_setup,
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::ProtocolHandler,
  },
};

generic_protocol_setup!(FeelingSo, "feelingso");

#[derive(Default)]
pub struct FeelingSo {}

impl ProtocolHandler for FeelingSo {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn handle_scalar_cmd(
    &self,
    commands: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let cmd1 = commands[0];
    let cmd2 = if commands.len() > 1 {
      commands[1]
    } else {
      None
    };

    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        0xaa,
        0x40,
        0x03,
        cmd1.unwrap_or((ActuatorType::Vibrate, 0)).1 as u8,
        cmd2.unwrap_or((ActuatorType::Oscillate, 0)).1 as u8,
        0x14, // Oscillate range: 1 to 4
        0x19, // Checksum?
      ],
      false,
    )
    .into()])
  }
}
