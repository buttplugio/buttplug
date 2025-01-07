// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
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

generic_protocol_setup!(NexusRevo, "nexus-revo");

#[derive(Default)]
pub struct NexusRevo {}

impl ProtocolHandler for NexusRevo {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0xaa, 0x01, 0x01, 0x00, 0x01, scalar as u8],
      true,
    )
    .into()])
  }

  fn handle_rotate_cmd(
    &self,
    commands: &[Option<(u32, bool)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if let Some(Some(cmd)) = commands.first() {
      return Ok(vec![HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![
          0xaa,
          0x01,
          0x02,
          0x00,
          cmd.0 as u8 + if cmd.0 != 0 && cmd.1 { 2 } else { 0 },
          0x00,
        ],
        true,
      )
      .into()]);
    }
    Ok(vec![])
  }
}
