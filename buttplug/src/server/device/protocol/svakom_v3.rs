// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::{device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  }, message::checked_actuator_cmd::CheckedActuatorCmdV4},
};

generic_protocol_setup!(SvakomV3, "svakom-v3");

#[derive(Default)]
pub struct SvakomV3 {}

impl ProtocolHandler for SvakomV3 {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

    fn handle_actuator_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      cmd.feature_id(),
      Endpoint::Tx,
      [
        0x55,
        if cmd.feature_index() == 0 { 0x03 } else { 0x09 },
        if cmd.feature_index() == 0 { 0x03 } else { 0x00 },
        0x00,
        if cmd.value() == 0 { 0x00 } else { 0x01 },
        cmd.value() as u8,
      ]
      .to_vec(),
      false,
    )
    .into()])
  }

  fn handle_value_rotate_cmd(
    &self,
    cmd: &CheckedActuatorCmdV4
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      cmd.feature_id(),
      Endpoint::Tx,
      [0x55, 0x08, 0x00, 0x00, cmd.value() as u8, 0xff].to_vec(),
      false,
    )
    .into()])
  }
}
