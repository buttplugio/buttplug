// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::handle_nonaggregate_vibrate_cmd;
use crate::{
  core::{errors::ButtplugDeviceError, messages::{self, Endpoint}},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(TCodeV03, "tcode-v03");

#[derive(Default)]
pub struct TCodeV03 {}

impl ProtocolHandler for TCodeV03 {
  fn handle_linear_cmd(
    &self,
    msg: messages::LinearCmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut msg_vec = vec![];
    for v in msg.vectors() {
      let position = (v.position * 99f64) as u32;

      let command = format!("L{}{:02}I{}\n", v.index, position, v.duration);
      msg_vec.push(HardwareWriteCmd::new(
        Endpoint::Tx,
        command.as_bytes().to_vec(),
        false,
      ).into());
    }
    Ok(msg_vec)
  }

  fn handle_vibrate_cmd(
    &self,
    cmds: &Vec<Option<u32>>,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(handle_nonaggregate_vibrate_cmd(cmds, |index, speed| {
      HardwareWriteCmd::new(
        Endpoint::Tx,
        format!("V{}{:02}\n", index, speed).as_bytes().to_vec(),
        false,
      ).into()
    }))
  }
}
