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

generic_protocol_setup!(TryFunBlackHole, "tryfun-blackhole");

#[derive(Default)]
pub struct TryFunBlackHole {
  packet_id: AtomicU8,
}

impl ProtocolHandler for TryFunBlackHole {
  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut sum: u8 = 0xff;
    let mut data = vec![
      self.packet_id.fetch_add(1, Ordering::Relaxed),
      0x02,
      0x00,
      0x03,
      0x0c,
      speed as u8,
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

  fn handle_output_temperature_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    level: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // TryFun Rock heat: sub-command 0x05 is a binary on/off toggle
    // (verified on hardware - no intermediate levels), so clamp the
    // normalized level to 0/1. Only devices whose config declares a
    // temperature feature (TF-ROCK) reach this handler; TF-BHPLUS has no
    // heat hardware and is unaffected.
    let level = level.clamp(0, 1) as u8;
    let mut sum: u8 = 0xff;
    let mut data = vec![
      self.packet_id.fetch_add(1, Ordering::Relaxed),
      0x02,
      0x00,
      0x03,
      0x05,
      level,
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

  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut sum: u8 = 0xff;
    let mut data = vec![
      self.packet_id.fetch_add(1, Ordering::Relaxed),
      0x02,
      0x00,
      0x03,
      0x09,
      speed as u8,
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
