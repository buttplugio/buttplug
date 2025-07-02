// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
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
use std::num::Wrapping;

generic_protocol_setup!(Xibao, "xibao");

#[derive(Default)]
pub struct Xibao {}

impl ProtocolHandler for Xibao {
  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![
        0x66,
        0x3a,
        0x00,
        0x06,
        0x00,
        0x06,
        0x01,
        0x02,
        0x00,
        0x02,
        0x04,
        speed as u8,
        (Wrapping(speed as u8) + Wrapping(0xb5)).0,
      ],
      false,
    )
    .into()])
  }
}
