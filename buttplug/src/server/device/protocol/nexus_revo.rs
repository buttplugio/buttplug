// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::Uuid;

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(NexusRevo, "nexus-revo");

#[derive(Default)]
pub struct NexusRevo {}

impl ProtocolHandler for NexusRevo {


  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      feature_id,
      Endpoint::Tx,
      vec![0xaa, 0x01, 0x01, 0x00, 0x01, speed as u8],
      true,
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
    Ok(vec![HardwareWriteCmd::new(
      feature_id,
      Endpoint::Tx,
      vec![
        0xaa,
        0x01,
        0x02,
        0x00,
        speed as u8 + if speed != 0 && clockwise { 2 } else { 0 },
        0x00,
      ],
      true,
    )
    .into()])
  }
}
