// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::Endpoint,
  },
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(Yiciyuan, "yiciyuan");

#[derive(Default)]
pub struct Yiciyuan {}

impl ProtocolHandler for Yiciyuan {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut cmd = vec![0x35, 0x12, scalar as u8];
    cmd.resize(16, 0);
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      cmd,
      false,
    )
    .into()])
  }
}
