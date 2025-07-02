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

const JEJOUE_PROTOCOL_UUID: Uuid = uuid!("d3dd2bf5-b029-4bc1-9466-39f82c2e3258");
generic_protocol_setup!(JeJoue, "jejoue");

pub struct JeJoue {
  speeds: [AtomicU8; 2],
}

impl Default for JeJoue {
  fn default() -> Self {
    Self {
      speeds: [AtomicU8::new(0), AtomicU8::new(0)],
    }
  }
}

impl ProtocolHandler for JeJoue {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);

    // Default to both vibes
    let mut pattern: u8 = 1;

    // Use vibe 1 as speed
    let mut speed = self.speeds[0].load(Ordering::Relaxed);
    let vibe1_running = speed > 0;
    let mut vibe2_running = false;
    // Unless it's zero, then give vibe 2 a chance
    if !vibe1_running {
      speed = self.speeds[1].load(Ordering::Relaxed);

      // If we've vibing on 2 only, then change the pattern
      if speed != 0 {
        vibe2_running = true;
        pattern = 3;
      }
    }

    // If we've vibing on 1 only, then change the pattern
    if pattern == 1 && speed != 0 && !vibe2_running {
      pattern = 2;
    }

    Ok(vec![HardwareWriteCmd::new(
      &[JEJOUE_PROTOCOL_UUID],
      Endpoint::Tx,
      vec![pattern, speed],
      false,
    )
    .into()])
  }
}
