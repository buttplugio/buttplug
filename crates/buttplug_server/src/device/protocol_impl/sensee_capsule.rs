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

generic_protocol_setup!(SenseeCapsule, "sensee-capsule");

#[derive(Default)]
pub struct SenseeCapsule {}

impl ProtocolHandler for SenseeCapsule {
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
        0x55,
        0xaa,
        0xf0,
        0x01,
        0x00,
        0x12,
        0x66,
        0xf9,
        0xf0 | speed as u8,
      ],
      false,
    )
    .into()])
  }

  fn handle_output_constrict_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![
        0x55,
        0xaa,
        0xf0,
        0x01,
        0x00,
        0x11,
        0x66,
        0xf2,
        0xf0 | level as u8,
        0x00,
        0x00,
      ],
      false,
    )
    .into()])
  }
}
