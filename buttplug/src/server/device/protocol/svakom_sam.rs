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

generic_protocol_setup!(SvakomSam, "svakom-sam");

#[derive(Default)]
pub struct SvakomSam {}

impl ProtocolHandler for SvakomSam {
  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut msg_vec = vec![];
    if let Some((_, speed)) = cmds[0] {
      msg_vec.push(
        HardwareWriteCmd::new(Endpoint::Tx, [18, 1, 3, 0, 5, speed as u8].to_vec(), true).into(),
      );
    }
    if cmds.len() > 1 {
      if let Some((_, speed)) = cmds[1] {
        msg_vec
          .push(HardwareWriteCmd::new(Endpoint::Tx, [18, 6, 1, speed as u8].to_vec(), true).into());
      }
    }
    Ok(msg_vec)
  }
}
