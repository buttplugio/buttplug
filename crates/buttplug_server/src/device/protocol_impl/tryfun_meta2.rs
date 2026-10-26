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

    Ok(vec![
      HardwareWriteCmd::new(&[feature_id], Endpoint::Tx, data, false).into(),
    ])
  }

  fn handle_output_rotate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // Vendor app encoding, verified on hardware against the vendor app's
    // rotaryToBLE (2026-10-26): positive speed maps to byte 0..100, negative
    // speed maps to the vendor's reverse band 155..255 via (255 + v) & 0xFF.
    // The previous transform inverted the direction relative to the vendor
    // app: upstream +50 produced byte 0xCD, which the firmware treats as
    // vendor -50 (the device rotated the opposite way from the vendor app).
    let speed: u8 = if speed >= 0 {
      speed.clamp(0, 100) as u8
    } else {
      (255 + speed.clamp(-100, 0)) as u8
    };
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

    Ok(vec![
      HardwareWriteCmd::new(&[feature_id], Endpoint::Tx, data, false).into(),
    ])
  }
}
