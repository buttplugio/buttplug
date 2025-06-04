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
  }, message::checked_actuator_cmd::CheckedActuatorCmdV4},
};

generic_protocol_setup!(LoveNuts, "lovenuts");

#[derive(Default)]
pub struct LoveNuts {}

impl ProtocolHandler for LoveNuts {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_value_vibrate_cmd(
    &self,
    cmd: &CheckedActuatorCmdV4
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut data: Vec<u8> = vec![0x45, 0x56, 0x4f, 0x4c];
    data.append(&mut [cmd.value() as u8 | (cmd.value() as u8) << 4; 10].to_vec());
    data.push(0x00);
    data.push(0xff);

    Ok(vec![HardwareWriteCmd::new(cmd.feature_id(), Endpoint::Tx, data, false).into()])
  }
}
