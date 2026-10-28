// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::Uuid;

use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, generic_protocol_setup},
};
use std::sync::atomic::{AtomicU8, Ordering};

generic_protocol_setup!(TryFunRock, "tryfun-rock");

#[derive(Default)]
pub struct TryFunRock {
  packet_id: AtomicU8,
}

impl TryFunRock {
  fn format_frame(
    &self,
    feature_id: Uuid,
    sub_command: u8,
    value: u8,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut sum: u8 = 0xff;
    let mut data = vec![
      self.packet_id.fetch_add(1, Ordering::Relaxed),
      0x02,
      0x00,
      0x03,
      sub_command,
      value,
    ];
    let mut count = 1;
    for item in data.iter().skip(1) {
      sum -= item;
      count += 1;
    }
    sum += count;
    data.push(sum);

    Ok(vec![
      HardwareWriteCmd::new(&[feature_id], Endpoint::Tx, data, false).into(),
    ])
  }
}

impl ProtocolHandler for TryFunRock {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.format_frame(feature_id, 0x09, speed as u8)
  }

  fn handle_output_temperature_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    level: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // TryFun Rock heat: sub-command 0x05 is a binary on/off toggle
    // (verified on hardware - no intermediate levels), so clamp the
    // normalized level to 0/1.
    self.format_frame(feature_id, 0x05, level.clamp(0, 1) as u8)
  }
}
