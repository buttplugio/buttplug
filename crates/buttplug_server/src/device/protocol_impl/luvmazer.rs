// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::Uuid;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, generic_protocol_setup}
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;


generic_protocol_setup!(Luvmazer, "luvmazer");

#[derive(Default)]
pub struct Luvmazer {}

impl ProtocolHandler for Luvmazer {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![0xa0, 0x01, 0x00, 0x00, 0x64, speed as u8],
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
      vec![0xa0, 0x0f, 0x00, 0x00, 0x64, speed as u8],
      false,
    )
    .into()])
  }
}
