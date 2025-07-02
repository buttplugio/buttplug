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

generic_protocol_setup!(LoveNuts, "lovenuts");

#[derive(Default)]
pub struct LoveNuts {}

impl ProtocolHandler for LoveNuts {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut data: Vec<u8> = vec![0x45, 0x56, 0x4f, 0x4c];
    data.append(&mut [speed as u8 | (speed as u8) << 4; 10].to_vec());
    data.push(0x00);
    data.push(0xff);

    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      data,
      false,
    )
    .into()])
  }
}
