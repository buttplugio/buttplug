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

generic_protocol_setup!(Picobong, "picobong");

#[derive(Default)]
pub struct Picobong {}

impl ProtocolHandler for Picobong {
  fn handle_vibrate_cmd(
    &self,
    cmds: &Vec<Option<u32>>,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(handle_nonaggregate_vibrate_cmd(cmds, |_, speed| {
      let mode: u8 = if speed == 0 { 0xff } else { 0x01 };
      HardwareWriteCmd::new(Endpoint::Tx, [0x01, mode, speed as u8].to_vec(), false).into()
    }))
  }
}

// TODO Write tests for protocol
