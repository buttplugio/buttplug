// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::ActuatorType;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(JoyHub, "joyhub");

#[derive(Default)]
pub struct JoyHub {}

impl ProtocolHandler for JoyHub {
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
        0xa0,
        0x03,
        cmd1.unwrap_or((ActuatorType::Oscillate, 0)).1 as u8,
        0x00,
        cmd2.unwrap_or((ActuatorType::Oscillate, 0)).1 as u8,
        0x00,
        0xaa,
      ],
      false,
    )
    .into()])
  }
}
