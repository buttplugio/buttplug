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

generic_protocol_setup!(LiboShark, "libo-shark");

#[derive(Default)]
pub struct LiboShark {}

impl ProtocolHandler for LiboShark {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // Store off result before the match, so we drop the lock ASAP.
    let mut data = 0u8;
    if let Some((_, speed)) = cmds[0] {
      data |= (speed as u8) << 4;
    }
    if let Some((_, speed)) = cmds[1] {
      data |= speed as u8;
    }
    Ok(vec![
      HardwareWriteCmd::new(Endpoint::Tx, vec![data], false).into()
    ])
  }
}
