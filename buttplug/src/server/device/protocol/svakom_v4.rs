// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::ActuatorType;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(SvakomV4, "svakom-v4");

#[derive(Default)]
pub struct SvakomV4 {}

impl ProtocolHandler for SvakomV4 {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn handle_scalar_cmd(
    &self,
    commands: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut actuator: u8 = 0;
    let mut scalar: u8 = 0;
    for i in 0..commands.len() {
      if let Some(cmd) = commands[i] {
        if cmd.1 != 0 && scalar == 0 && commands.len() > 1 {
          actuator = i as u8 + 1; // just this actuators
        } else if cmd.1 != 0 {
          actuator = 0; // all the actuators
        }
        scalar = u8::max(scalar, cmd.1 as u8); // max of all actuators
      }
    }

    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      [
        0x55,
        0x03,
        actuator,
        0x00,
        if scalar == 0 { 0x00 } else { 0x01 },
        scalar as u8,
      ]
      .to_vec(),
      false,
    )
    .into()])
  }
}
