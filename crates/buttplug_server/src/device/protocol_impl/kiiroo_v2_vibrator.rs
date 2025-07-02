// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::atomic::{AtomicU8, Ordering};

use uuid::Uuid;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

generic_protocol_setup!(KiirooV2Vibrator, "kiiroo-v2-vibrator");

pub struct KiirooV2Vibrator {
  speeds: [AtomicU8; 3],
}

impl Default for KiirooV2Vibrator {
  fn default() -> Self {
    Self {
      speeds: [AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0)],
    }
  }
}

impl ProtocolHandler for KiirooV2Vibrator {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      self
        .speeds
        .iter()
        .map(|v| v.load(Ordering::Relaxed))
        .collect(),
      false,
    )
    .into()])
  }
}
