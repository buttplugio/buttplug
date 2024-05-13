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

generic_protocol_setup!(ActiveJoy, "activejoy");

#[derive(Default)]
pub struct ActiveJoy {}

impl ProtocolHandler for ActiveJoy {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      [
        0xb0,        // static header
        0x01,        // mode: 1=vibe, 5=shock, 6=thrust, 7=suction, 8=rotation, 16=swing,
        0x00,        // strong mode = 1 (thrust, suction, swing, rotate)
        index as u8, // 0 unless vibe2
        if scalar == 0 { 0x00 } else { 0x01 },
        scalar as u8,
      ]
      .to_vec(),
      false,
    )
    .into()])
  }
}
