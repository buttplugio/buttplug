// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::atomic::{AtomicU8, Ordering};

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::{device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  }, message::checked_value_cmd::CheckedValueCmdV4},
};

generic_protocol_setup!(HtkBm, "htk_bm");

pub struct HtkBm {
  speeds: [AtomicU8; 2]
}

impl Default for HtkBm {
  fn default() -> Self {
    Self {
      speeds: [AtomicU8::new(0), AtomicU8::new(0)]
    }
  }
}

impl ProtocolHandler for HtkBm {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_value_vibrate_cmd(
    &self,
    cmd: &CheckedValueCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut cmd_vec = vec![];
    self.speeds[cmd.feature_index() as usize].store(cmd.value() as u8, Ordering::Relaxed);

    let mut data: u8 = 15;
    let left = self.speeds[0].load(Ordering::Relaxed);
    let right = self.speeds[1].load(Ordering::Relaxed);
    if left != 0 && right != 0 {
      data = 11 // both (normal mode)
    } else if left != 0 {
      data = 12 // left only
    } else if right != 0 {
      data = 13 // right only
    }
    cmd_vec.push(HardwareWriteCmd::new(Endpoint::Tx, vec![data], false).into());
    Ok(cmd_vec)
  }
}
