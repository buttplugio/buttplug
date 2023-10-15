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

generic_protocol_setup!(MagicMotionV4, "magic-motion-4");

#[derive(Default)]
pub struct MagicMotionV4 {}

impl ProtocolHandler for MagicMotionV4 {
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
    let data = if cmds.len() == 1 {
      vec![
        0x10,
        0xff,
        0x04,
        0x0a,
        0x32,
        0x32,
        0x00,
        0x04,
        0x08,
        cmds[0].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8,
        0x64,
        0x00,
        0x04,
        0x08,
        cmds[0].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8,
        0x64,
        0x01,
      ]
    } else {
      vec![
        0x10,
        0xff,
        0x04,
        0x0a,
        0x32,
        0x32,
        0x00,
        0x04,
        0x08,
        cmds[0].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8,
        0x64,
        0x00,
        0x04,
        0x08,
        cmds[1].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8,
        0x64,
        0x01,
      ]
    };
    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, true).into()])
  }
}
