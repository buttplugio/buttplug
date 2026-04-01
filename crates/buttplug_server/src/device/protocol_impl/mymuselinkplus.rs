// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
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

generic_protocol_setup!(MyMuseLinkPlus, "mymuselinkplus");

#[derive(Default)]
pub struct MyMuseLinkPlus {}

impl ProtocolHandler for MyMuseLinkPlus {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let data = if speed == 0 {
      vec![0xAA, 0x55, 0x06, 0xAA, 0x00, 0x00, 0x00, 0x00]
    } else {
      vec![0xAA, 0x55, 0x06, 0x01, 0x01, 0x01, speed as u8, 0xFF]
    };

    Ok(vec![
      HardwareWriteCmd::new(&[feature_id], Endpoint::Tx, data, false).into(),
    ])
  }
}
