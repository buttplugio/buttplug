// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(JeJoue, "jejoue");

#[derive(Default)]
pub struct JeJoue {}

impl ProtocolHandler for JeJoue {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // Store off result before the match, so we drop the lock ASAP.
    // Default to both vibes
    let mut pattern: u8 = 1;

    // Use vibe 1 as speed
    let mut speed = cmds[0].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8;

    // Unless it's zero, then five vibe 2 a chance
    if speed == 0 {
      speed = cmds[1].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8;

      // If we've vibing on 2 only, then change the pattern
      if speed != 0 {
        pattern = 3;
      }
    }

    // If we've vibing on 1 only, then change the pattern
    if pattern == 1 && speed != 0 && cmds[1].unwrap_or((ActuatorType::Vibrate, 0)).1 == 0 {
      pattern = 2;
    }
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![pattern, speed],
      false,
    )
    .into()])
  }
}
