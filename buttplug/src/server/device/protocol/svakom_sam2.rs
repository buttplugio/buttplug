// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
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

generic_protocol_setup!(SvakomSam2, "svakom-sam2");

#[derive(Default)]
pub struct SvakomSam2 {}

impl ProtocolHandler for SvakomSam2 {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      [
        0x55,
        0x03,
        0x00,
        0x00,
        if scalar == 0 { 0x00 } else { 0x05 },
        scalar as u8,
        0x00,
      ]
      .to_vec(),
      true,
    )
    .into()])
  }

  fn handle_scalar_constrict_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      [
        0x55,
        0x09,
        0x00,
        0x00,
        if scalar == 0 { 0x00 } else { 0x01 },
        scalar as u8,
        0x00,
      ]
      .to_vec(),
      true,
    )
    .into()])
  }
}
