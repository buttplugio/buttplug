// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::handle_nonaggregate_vibrate_cmd;
use crate::{
  core::{errors::ButtplugDeviceError, messages::Endpoint},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(Hismith, "hismith");

#[derive(Default)]
pub struct Hismith {}

impl ProtocolHandler for Hismith {
  fn handle_vibrate_cmd(
    &self,
    cmds: &Vec<Option<u32>>,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(handle_nonaggregate_vibrate_cmd(cmds, |_, speed| {
      HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![0xAA, 0x04, speed as u8, (speed + 4) as u8],
        false,
      )
      .into()
    }))
  }
}
