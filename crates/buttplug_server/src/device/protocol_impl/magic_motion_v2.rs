// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::atomic::{AtomicU8, Ordering};

use uuid::{uuid, Uuid};

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

const MAGIC_MOTION_2_PROTOCOL_UUID: Uuid = uuid!("4d6e9297-c57e-4ce7-a63c-24cc7d117a47");

generic_protocol_setup!(MagicMotionV2, "magic-motion-2");

pub struct MagicMotionV2 {
  speeds: [AtomicU8; 2],
}

impl Default for MagicMotionV2 {
  fn default() -> Self {
    Self {
      speeds: [AtomicU8::new(0), AtomicU8::new(0)],
    }
  }
}

impl ProtocolHandler for MagicMotionV2 {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    let data = vec![
      0x10,
      0xff,
      0x04,
      0x0a,
      0x32,
      0x0a,
      0x00,
      0x04,
      0x08,
      self.speeds[0].load(Ordering::Relaxed),
      0x64,
      0x00,
      0x04,
      0x08,
      self.speeds[1].load(Ordering::Relaxed),
      0x64,
      0x01,
    ];
    Ok(vec![HardwareWriteCmd::new(
      &[MAGIC_MOTION_2_PROTOCOL_UUID],
      Endpoint::Tx,
      data,
      false,
    )
    .into()])
  }
}
