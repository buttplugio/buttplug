// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(SexverseLG389, "sexverse-lg389");

#[derive(Default)]
pub struct SexverseLG389 {}

impl ProtocolHandler for SexverseLG389 {
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
    let vibe = commands[0].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8;
    let osc = if commands.len() > 1 {
      commands[1].unwrap_or((ActuatorType::Oscillate, 0)).1 as u8
    } else {
      0
    };
    let range = if osc == 0 { 0 } else { 4u8 }; // Full range
    let anchor = if osc == 0 { 0 } else { 1u8 }; // Anchor to base
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0xaa, 0x05, vibe, 0x14, anchor, 0x00, range, 0x00, osc, 0x00],
      true,
    )
    .into()])
  }
}
