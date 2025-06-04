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

generic_protocol_setup!(Maxpro, "maxpro");

#[derive(Default)]
pub struct Maxpro {}

impl ProtocolHandler for Maxpro {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

    fn handle_value_vibrate_cmd(
    &self,
    cmd: &CheckedActuatorCmdV4
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut data = vec![
      0x55u8,
      0x04,
      0x07,
      0xff,
      0xff,
      0x3f,
      cmd.value() as u8,
      0x5f,
      cmd.value() as u8,
      0x00,
    ];
    let mut crc: u8 = 0;

    for b in data.clone() {
      crc = crc.wrapping_add(b);
    }

    data[9] = crc;
    Ok(vec![HardwareWriteCmd::new(cmd.feature_id(), Endpoint::Tx, data, false).into()])
  }
}
