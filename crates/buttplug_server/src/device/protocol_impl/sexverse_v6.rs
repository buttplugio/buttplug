// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::atomic::{AtomicU8, Ordering};
use uuid::{uuid, Uuid};

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, generic_protocol_setup},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

generic_protocol_setup!(SexverseV6, "sexverse-v6");

const SEXVERSE_PROTOCOL_UUID: Uuid = uuid!("f38ceb5e-84b4-4475-9119-6e48f163c4ec");

#[derive(Default)]
pub struct SexverseV6 {
  vibe_speed: AtomicU8,
  osc_speed: AtomicU8,
  suck_speed: AtomicU8,
}

impl SexverseV6 {
  fn generate_command(&self) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let vibe = self.vibe_speed.load(Ordering::Relaxed);
    let osc = self.osc_speed.load(Ordering::Relaxed);
    let suck = self.suck_speed.load(Ordering::Relaxed);
    Ok(vec![
      HardwareWriteCmd::new(
        &[SEXVERSE_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![0xaa, 0x03, 0x03, vibe, osc, suck],
        false,
      )
          .into(),
    ])
  }
}

impl ProtocolHandler for SexverseV6 {
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
  fn handle_output_constrict_cmd(
    &self,
    _feature_index: u32,
    _feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.suck_speed.store(speed as u8, Ordering::Relaxed);
    self.generate_command()
  }
}
