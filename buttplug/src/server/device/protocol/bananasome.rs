// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{
      ActuatorType,
      ActuatorType::{Oscillate, Vibrate},
      Endpoint,
    },
  },
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(Bananasome, "bananasome");

#[derive(Default)]
pub struct Bananasome {}

impl ProtocolHandler for Bananasome {
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
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        0xa0,
        0x03,
        commands[0].unwrap_or((Oscillate, 0)).1 as u8,
        if commands.len() > 1 {
          commands[1].unwrap_or((Vibrate, 0)).1
        } else {
          0
        } as u8,
        if commands.len() > 2 {
          commands[2].unwrap_or((Vibrate, 0)).1
        } else {
          0
        } as u8,
      ],
      false,
    )
    .into()])
  }
}
