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

generic_protocol_setup!(Picobong, "picobong");

#[derive(Default)]
pub struct Picobong {}

impl ProtocolHandler for Picobong {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mode: u8 = if scalar == 0 { 0xff } else { 0x01 };
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      [0x01, mode, scalar as u8].to_vec(),
      false,
    )
    .into()])
  }
}

// TODO Write tests for protocol
