// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

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
  }, message::{checked_value_cmd::CheckedValueCmdV4, checked_value_with_parameter_cmd::CheckedValueWithParameterCmdV4}},
};

generic_protocol_setup!(Motorbunny, "motorbunny");

#[derive(Default)]
pub struct Motorbunny {}

impl ProtocolHandler for Motorbunny {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_value_vibrate_cmd(
    &self,
    cmd: &CheckedValueCmdV4
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut command_vec: Vec<u8>;
    if cmd.value() == 0 {
      command_vec = vec![0xf0, 0x00, 0x00, 0x00, 0x00, 0xec];
    } else {
      command_vec = vec![0xff];
      let mut vibe_commands = [cmd.value() as u8, 0x14].repeat(7);
      let crc = vibe_commands
        .iter()
        .fold(0u8, |a, b| a.overflowing_add(*b).0);
      command_vec.append(&mut vibe_commands);
      command_vec.append(&mut vec![crc, 0xec]);
    }
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      command_vec,
      false,
    )
    .into()])
  }

  fn handle_rotation_with_direction_cmd(
    &self,
    cmd: &CheckedValueWithParameterCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut command_vec: Vec<u8>;
    if cmd.value() == 0 {
      command_vec = vec![0xa0, 0x00, 0x00, 0x00, 0x00, 0xec];
    } else {
      command_vec = vec![0xaf];
      let mut rotate_command = [if cmd.parameter() > 0 { 0x2a } else { 0x29 }, cmd.value() as u8].repeat(7);
      let crc = rotate_command
        .iter()
        .fold(0u8, |a, b| a.overflowing_add(*b).0);
      command_vec.append(&mut rotate_command);
      command_vec.append(&mut vec![crc, 0xec]);
    }
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      command_vec,
      false,
    )
    .into()])
  }
}
