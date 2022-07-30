// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, messages::Endpoint},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(Nobra, "nobra");

#[derive(Default)]
pub struct Nobra {}

impl ProtocolHandler for Nobra {
  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let output_speed = if scalar == 0 { 0x70 } else { 0x60 + scalar };
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![output_speed as u8],
      false,
    )
    .into()])
  }
}
