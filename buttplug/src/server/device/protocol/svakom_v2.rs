// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
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

generic_protocol_setup!(SvakomV2, "svakom-v2");

#[derive(Default)]
pub struct SvakomV2 {}

impl ProtocolHandler for SvakomV2 {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_value_vibrate_cmd(
    &self,
    index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if index == 1 {
      Ok(vec![HardwareWriteCmd::new(
        Endpoint::Tx,
        [0x55, 0x06, 0x01, 0x00, scalar as u8, scalar as u8].to_vec(),
        true,
      )
      .into()])
    } else {
      Ok(vec![HardwareWriteCmd::new(
        Endpoint::Tx,
        [
          0x55,
          0x03,
          0x03,
          0x00,
          if scalar == 0 { 0x00 } else { 0x01 },
          scalar as u8,
        ]
        .to_vec(),
        true,
      )
      .into()])
    }
  }
}
