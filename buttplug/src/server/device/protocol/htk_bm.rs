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

generic_protocol_setup!(HtkBm, "htk_bm");

#[derive(Default)]
pub struct HtkBm {}

impl ProtocolHandler for HtkBm {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut cmd_vec = vec![];
    if cmds.len() == 2 {
      let mut data: u8 = 15;
      let left = cmds[0].unwrap_or((ActuatorType::Vibrate, 0)).1;
      let right = cmds[1].unwrap_or((ActuatorType::Vibrate, 0)).1;
      if left != 0 && right != 0 {
        data = 11 // both (normal mode)
      } else if left != 0 {
        data = 12 // left only
      } else if right != 0 {
        data = 13 // right only
      }
      cmd_vec.push(HardwareWriteCmd::new(Endpoint::Tx, vec![data], false).into());
    }
    Ok(cmd_vec)
  }
}
