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

const BANANASOME_PROTOCOL_UUID: Uuid = uuid!("a0a2e5f8-3692-4f6b-8add-043513ed86f6");
generic_protocol_setup!(Bananasome, "bananasome");

pub struct Bananasome {
  current_commands: [AtomicU8; 3],
}

impl Default for Bananasome {
  fn default() -> Self {
    Self {
      current_commands: [AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0)],
    }
  }
}

impl Bananasome {
  fn hardware_command(&self, feature_index: u32, speed: u32) -> Vec<HardwareCommand> {
    self.current_commands[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    vec![HardwareWriteCmd::new(
      &[BANANASOME_PROTOCOL_UUID],
      Endpoint::Tx,
      vec![
        0xa0,
        0x03,
        self.current_commands[0].load(Ordering::Relaxed),
        self.current_commands[1].load(Ordering::Relaxed),
        self.current_commands[2].load(Ordering::Relaxed),
      ],
      false,
    )
    .into()]
  }
}

impl ProtocolHandler for Bananasome {
  fn handle_output_oscillate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(self.hardware_command(feature_index, speed))
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(self.hardware_command(feature_index, speed))
  }
}
