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

generic_protocol_setup!(MetaXSireV4, "metaxsire-v4");

#[derive(Default)]
pub struct MetaXSireV4 {}

impl ProtocolHandler for MetaXSireV4 {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0xbb, 0x01, speed as u8, 0x66],
        true,
      )
      .into(),
    ])
  }
}
