// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
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

generic_protocol_setup!(SexverseLG389, "sexverse-lg389");

const SEXVERSE_PROTOCOL_UUID: Uuid = uuid!("575b2394-8f88-4367-a355-11321efda686");

#[derive(Default)]
pub struct SexverseLG389 {
  vibe_speed: AtomicU8,
  osc_speed: AtomicU8,
}

impl SexverseLG389 {
  fn generate_command(&self) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let vibe = self.vibe_speed.load(Ordering::Relaxed);
    let osc = self.osc_speed.load(Ordering::Relaxed);
    let range = if osc == 0 { 0 } else { 4u8 }; // Full range
    let anchor = if osc == 0 { 0 } else { 1u8 }; // Anchor to base
    Ok(vec![HardwareWriteCmd::new(
      &[SEXVERSE_PROTOCOL_UUID],
      Endpoint::Tx,
      vec![0xaa, 0x05, vibe, 0x14, anchor, 0x00, range, 0x00, osc, 0x00],
      true,
    )
    .into()])
  }
}

impl ProtocolHandler for SexverseLG389 {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.vibe_speed.store(speed as u8, Ordering::Relaxed);
    self.generate_command()
  }

  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.osc_speed.store(speed as u8, Ordering::Relaxed);
    self.generate_command()
  }
}
