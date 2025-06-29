// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::Uuid;

use buttplug_core::{errors::ButtplugDeviceError, message::Endpoint};
use crate::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
};

generic_protocol_setup!(Sakuraneko, "sakuraneko");

#[derive(Default)]
pub struct Sakuraneko {}

impl ProtocolHandler for Sakuraneko {


  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![
        0xa1,
        0x08,
        0x01,
        0x00,
        0x00,
        0x00,
        0x64,
        speed as u8,
        0x00,
        0x64,
        0xdf,
        0x55,
      ],
      false,
    )
    .into()])
  }

  fn handle_output_rotate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![
        0xa2,
        0x08,
        0x01,
        0x00,
        0x00,
        0x00,
        0x64,
        speed as u8,
        0x00,
        0x32,
        0xdf,
        0x55,
      ],
      false,
    )
    .into()])
  }
}
