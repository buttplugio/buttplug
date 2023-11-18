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

generic_protocol_setup!(LiboVibes, "libo-vibes");

#[derive(Default)]
pub struct LiboVibes {}

impl ProtocolHandler for LiboVibes {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut msg_vec = vec![];
    for (index, cmd) in cmds.iter().enumerate() {
      if let Some((_, speed)) = cmd {
        if index == 0 {
          msg_vec.push(HardwareWriteCmd::new(Endpoint::Tx, vec![*speed as u8], false).into());

          // If this is a single vibe device, we need to send stop to TxMode too
          if *speed as u8 == 0 && cmds.len() == 1 {
            msg_vec.push(HardwareWriteCmd::new(Endpoint::TxMode, vec![0u8], false).into());
          }
        } else if index == 1 {
          msg_vec.push(HardwareWriteCmd::new(Endpoint::TxMode, vec![*speed as u8], false).into());
        }
      }
    }
    Ok(msg_vec)
  }
}
