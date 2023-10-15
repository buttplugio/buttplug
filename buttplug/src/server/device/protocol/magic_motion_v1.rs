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

generic_protocol_setup!(MagicMotionV1, "magic-motion-1");

#[derive(Default)]
pub struct MagicMotionV1 {}

impl ProtocolHandler for MagicMotionV1 {
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
      vec![
        0x0b,
        0xff,
        0x04,
        0x0a,
        0x32,
        0x32,
        0x00,
        0x04,
        0x08,
        scalar as u8,
        0x64,
        0x00,
      ],
      false,
    )
    .into()])
  }

  fn handle_scalar_oscillate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        0x0b,
        0xff,
        0x04,
        0x0a,
        0x32,
        0x32,
        0x00,
        0x04,
        0x08,
        scalar as u8,
        0x64,
        0x00,
      ],
      false,
    )
    .into()])
  }
}
