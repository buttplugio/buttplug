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

const FEELINGSO_PROTOCOL_UUID: Uuid = uuid!("397d4cce-3173-4f66-b7ad-6ee21e59f854");

generic_protocol_setup!(FeelingSo, "feelingso");

pub struct FeelingSo {
  speeds: [AtomicU8; 2],
}

impl Default for FeelingSo {
  fn default() -> Self {
    Self {
      speeds: [AtomicU8::new(0), AtomicU8::new(0)],
    }
  }
}

impl FeelingSo {
  fn hardware_command(&self) -> Vec<HardwareCommand> {
    vec![HardwareWriteCmd::new(
      &[FEELINGSO_PROTOCOL_UUID],
      Endpoint::Tx,
      vec![
        0xaa,
        0x40,
        0x03,
        self.speeds[0].load(Ordering::Relaxed),
        self.speeds[1].load(Ordering::Relaxed),
        0x14, // Oscillate range: 1 to 4
        0x19, // Checksum?
      ],
      false,
    )
    .into()]
  }
}

impl ProtocolHandler for FeelingSo {
  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[1].store(speed as u8, Ordering::Relaxed);
    Ok(self.hardware_command())
  }

  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[0].store(speed as u8, Ordering::Relaxed);
    Ok(self.hardware_command())
  }
}
