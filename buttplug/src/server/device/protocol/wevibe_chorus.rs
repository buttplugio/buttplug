// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
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

generic_protocol_setup!(WeVibeChorus, "wevibe-chorus");

#[derive(Default)]
pub struct WeVibeChorus {}

impl ProtocolHandler for WeVibeChorus {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let r_speed_int = cmds[0].unwrap_or((ActuatorType::Vibrate, 0u32)).1 as u8;
    let r_speed_ext = cmds
      .last()
      .unwrap_or(&None)
      .unwrap_or((ActuatorType::Vibrate, 0u32))
      .1 as u8;
    let data = if r_speed_int == 0 && r_speed_ext == 0 {
      vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    } else {
      // Note the motor order is flipped for the Chorus
      let status_byte: u8 =
        (if r_speed_ext == 0 { 0 } else { 2 }) | (if r_speed_int == 0 { 0 } else { 1 });
      vec![
        0x0f,
        0x03,
        0x00,
        r_speed_int,
        r_speed_ext,
        status_byte,
        0x00,
        0x00,
      ]
    };
    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, true).into()])
  }
}
