// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::Uuid;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, generic_protocol_setup},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

generic_protocol_setup!(Synchro, "synchro");

#[derive(Default)]
pub struct Synchro {}

impl ProtocolHandler for Synchro {
  fn handle_output_rotate_cmd(
      &self,
      _feature_index: u32,
      feature_id: Uuid,
      speed: i32,
    ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![
          0xa1,
          0x01,
          speed.unsigned_abs() as u8 | if speed >= 0 { 0x00 } else { 0x80 },
          0x77,
          0x55,
        ],
        false,
      )
      .into(),
    ])
  }
}
