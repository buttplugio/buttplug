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

generic_protocol_setup!(MagicMotionV3, "magic-motion-3");

#[derive(Default)]
pub struct MagicMotionV3 {}

impl ProtocolHandler for MagicMotionV3 {
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
        0x0b,
        0xff,
        0x04,
        0x0a,
        0x46,
        0x46,
        0x00,
        0x04,
        0x08,
        speed as u8,
        0x64,
        0x00,
      ],
      false,
    )
    .into()])
  }
}
