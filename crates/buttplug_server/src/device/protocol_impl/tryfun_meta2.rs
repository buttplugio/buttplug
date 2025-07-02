// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::Uuid;

use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};
use std::sync::atomic::{AtomicU8, Ordering};

generic_protocol_setup!(TryFunMeta2, "tryfun-meta2");

#[derive(Default)]
pub struct TryFunMeta2 {
  packet_id: AtomicU8,
}

impl ProtocolHandler for TryFunMeta2 {
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
      0x05,
      0x21,
      0x05,
      0x0b,
      speed as u8,
    ];
    let mut count = 1;
    for item in data.iter().skip(1) {
      sum -= item;
      count += 1;
    }
    sum += count;
    data.push(sum);

    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      data,
      false,
    )
    .into()])
  }

  fn handle_rotation_with_direction_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
    clockwise: bool,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut speed = speed as i8;
    if clockwise {
      speed += 1;
      speed *= -1;
    }
    let mut sum: u8 = 0xff;
    let mut data = vec![
      self.packet_id.fetch_add(1, Ordering::Relaxed),
      0x02,
      0x00,
      0x05,
      0x21,
      0x05,
      0x0e,
      speed as u8,
    ];
    let mut count = 1;
    for item in data.iter().skip(1) {
      sum = sum.wrapping_sub(*item);
      count += 1;
    }
    sum += count;
    data.push(sum);
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      data,
      false,
    )
    .into()])
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
      0x05,
      0x21,
      0x05,
      0x08,
      speed as u8,
    ];
    let mut count = 1;
    for item in data.iter().skip(1) {
      sum -= item;
      count += 1;
    }
    sum += count;
    data.push(sum);

    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      data,
      false,
    )
    .into()])
  }
}
