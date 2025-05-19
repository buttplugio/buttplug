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
    message::Endpoint,
  },
  server::{device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  }, message::checked_value_cmd::CheckedValueCmdV4},
};

generic_protocol_setup!(KiirooV2Vibrator, "kiiroo-v2-vibrator");

pub struct KiirooV2Vibrator {
  speeds: [AtomicU8; 3]
}

impl Default for KiirooV2Vibrator {
  fn default() -> Self {
    Self {
      speeds: [AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0)]
    }
  }
}

impl ProtocolHandler for KiirooV2Vibrator {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_value_vibrate_cmd(
    &self,
    cmd: &CheckedValueCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[cmd.feature_index() as usize].store(cmd.value() as u8, Ordering::Relaxed);
    Ok(vec![HardwareWriteCmd::new(
      cmd.feature_uuid(),
      Endpoint::Tx,
      self.speeds.iter().map(|v| v.load(Ordering::Relaxed)).collect(),
      false,
    )
    .into()])
  }
}
