// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::Endpoint,
  },
  server::{device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::ProtocolHandler,
  }, message::checked_value_cmd::CheckedValueCmdV4},
};

use super::generic_protocol_setup;

generic_protocol_setup!(Luvmazer, "luvmazer");

#[derive(Default)]
pub struct Luvmazer {
}

impl ProtocolHandler for Luvmazer {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

    fn handle_value_vibrate_cmd(
    &self,
    cmd: &CheckedValueCmdV4
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      cmd.feature_uuid(),
      Endpoint::Tx,
      vec![0xa0, 0x01, 0x00, 0x00, 0x64, cmd.value() as u8],
      false,
    )
    .into()])
  }

  fn handle_value_rotate_cmd(
    &self,
    cmd: &CheckedValueCmdV4
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      cmd.feature_uuid(),
      Endpoint::Tx,
      vec![0xa0, 0x0f, 0x00, 0x00, 0x64, cmd.value() as u8],
      false,
    )
    .into()])
  }
}
