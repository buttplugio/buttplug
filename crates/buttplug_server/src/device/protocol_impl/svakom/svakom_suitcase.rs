// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use uuid::Uuid;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};


generic_protocol_setup!(SvakomSuitcase, "svakom-suitcase");

#[derive(Default)]
pub struct SvakomSuitcase {}

impl ProtocolHandler for SvakomSuitcase {
  // I am like 90% sure this is wrong since this device has two vibrators, but the original
  // implementation made no sense in terms of knowing which command addressed which index. Putting
  // in a best effort here and we'll see if anyone complains.
  fn handle_output_vibrate_cmd(
      &self,
      _feature_index: u32,
      feature_id: Uuid,
      speed: u32,
    ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let scalar = speed;
    let mut speed = (scalar % 10) as u8;
    let mut intensity = if scalar == 0 {
      0u8
    } else {
      (scalar as f32 / 10.0).floor() as u8 + 1
    };
    if speed == 0 && intensity != 0 {
      // 10 -> 2,0 -> 1,A
      speed = 10;
      intensity -= 1;
    }
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      [0x55, 0x03, 0x00, 0x00, intensity, speed].to_vec(),
      false,
    ).into()])
  }
}
