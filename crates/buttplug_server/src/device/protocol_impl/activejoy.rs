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
generic_protocol_setup!(ActiveJoy, "activejoy");

#[derive(Default)]
pub struct ActiveJoy {}

impl ProtocolHandler for ActiveJoy {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      [
        0xb0,                // static header
        0x01,                // mode: 1=vibe, 5=shock, 6=thrust, 7=suction, 8=rotation, 16=swing,
        0x00,                // strong mode = 1 (thrust, suction, swing, rotate)
        feature_index as u8, // 0 unless vibe2
        if speed == 0 { 0x00 } else { 0x01 },
        speed as u8,
      ]
      .to_vec(),
      false,
    )
    .into()])
  }
}
