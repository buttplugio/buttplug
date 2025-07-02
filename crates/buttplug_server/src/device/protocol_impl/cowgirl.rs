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

const COWGIRL_PROTOCOL_UUID: Uuid = uuid!("0474d2fd-f566-4bed-8770-88e457a96144");
generic_protocol_setup!(Cowgirl, "cowgirl");

pub struct Cowgirl {
  speeds: [AtomicU8; 2],
}

impl Default for Cowgirl {
  fn default() -> Self {
    Self {
      speeds: [AtomicU8::new(0), AtomicU8::new(0)],
    }
  }
}

impl Cowgirl {
  fn hardware_commands(&self) -> Vec<HardwareCommand> {
    vec![HardwareWriteCmd::new(
      &[COWGIRL_PROTOCOL_UUID],
      Endpoint::Tx,
      vec![
        0x00,
        0x01,
        self.speeds[0].load(Ordering::Relaxed),
        self.speeds[1].load(Ordering::Relaxed),
      ],
      true,
    )
    .into()]
  }
}

impl ProtocolHandler for Cowgirl {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[0].store(speed as u8, Ordering::Relaxed);
    Ok(self.hardware_commands())
  }

  fn handle_output_rotate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[1].store(speed as u8, Ordering::Relaxed);
    Ok(self.hardware_commands())
  }
}
