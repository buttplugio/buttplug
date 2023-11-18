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

generic_protocol_setup!(Synchro, "synchro");

#[derive(Default)]
pub struct Synchro {}

impl ProtocolHandler for Synchro {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_rotate_cmd(
    &self,
    cmds: &[Option<(u32, bool)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if let Some(Some((speed, clockwise))) = cmds.get(0) {
      Ok(vec![HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![
          0xa1,
          0x01,
          *speed as u8
            | if *clockwise || *speed == 0 {
              0x00
            } else {
              0x80
            },
          0x77,
          0x55,
        ],
        false,
      )
      .into()])
    } else {
      Ok(vec![])
    }
  }
}
