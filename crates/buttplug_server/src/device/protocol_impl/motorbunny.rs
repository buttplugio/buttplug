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

use uuid::Uuid;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

generic_protocol_setup!(Motorbunny, "motorbunny");

#[derive(Default)]
pub struct Motorbunny {}

impl ProtocolHandler for Motorbunny {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut command_vec: Vec<u8>;
    if speed == 0 {
      command_vec = vec![0xf0, 0x00, 0x00, 0x00, 0x00, 0xec];
    } else {
      command_vec = vec![0xff];
      let mut vibe_commands = [speed as u8, 0x14].repeat(7);
      let crc = vibe_commands
        .iter()
        .fold(0u8, |a, b| a.overflowing_add(*b).0);
      command_vec.append(&mut vibe_commands);
      command_vec.append(&mut vec![crc, 0xec]);
    }
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      command_vec,
      false,
    )
    .into()])
  }

  fn handle_rotation_with_direction_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
    clockwise: bool,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut command_vec: Vec<u8>;
    if speed == 0 {
      command_vec = vec![0xa0, 0x00, 0x00, 0x00, 0x00, 0xec];
    } else {
      command_vec = vec![0xaf];
      let mut rotate_command = [if clockwise { 0x2a } else { 0x29 }, speed as u8].repeat(7);
      let crc = rotate_command
        .iter()
        .fold(0u8, |a, b| a.overflowing_add(*b).0);
      command_vec.append(&mut rotate_command);
      command_vec.append(&mut vec![crc, 0xec]);
    }
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      command_vec,
      false,
    )
    .into()])
  }
}
