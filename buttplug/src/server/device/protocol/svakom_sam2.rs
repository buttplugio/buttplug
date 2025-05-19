// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::{device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  }, message::checked_value_cmd::CheckedValueCmdV4},
};

generic_protocol_setup!(SvakomSam2, "svakom-sam2");

#[derive(Default)]
pub struct SvakomSam2 {}

impl ProtocolHandler for SvakomSam2 {
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
      [
        0x55,
        0x03,
        0x00,
        0x00,
        if cmd.value() == 0 { 0x00 } else { 0x05 },
        cmd.value() as u8,
        0x00,
      ]
      .to_vec(),
      true,
    )
    .into()])
  }

  fn handle_value_constrict_cmd(
    &self,
    cmd: &CheckedValueCmdV4
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      cmd.feature_uuid(),
      Endpoint::Tx,
      [
        0x55,
        0x09,
        0x00,
        0x00,
        if cmd.value() == 0 { 0x00 } else { 0x01 },
        cmd.value() as u8,
        0x00,
      ]
      .to_vec(),
      true,
    )
    .into()])
  }
}
