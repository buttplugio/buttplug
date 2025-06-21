// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::{uuid, Uuid};

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::Endpoint,
  }, generic_protocol_setup, server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{
      ProtocolHandler, ProtocolKeepaliveStrategy,
    },
  }
};
use std::sync::atomic::{AtomicU8, Ordering};

generic_protocol_setup!(SvakomV6, "svakom-v6");

const SVAKOM_V6_VIBRATOR_UUID: Uuid = uuid!("4cf33d95-a3d1-4ed4-9ac6-9ba6d6ccb091");

#[derive(Default)]
pub struct SvakomV6 {
  last_vibrator_speeds: [AtomicU8; 3],
}

impl ProtocolHandler for SvakomV6 {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {    
    self.last_vibrator_speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    if feature_index < 2 {
      let vibe1 = self.last_vibrator_speeds[0].load(Ordering::Relaxed);
      let vibe2 = self.last_vibrator_speeds[1].load(Ordering::Relaxed);
      Ok(vec![HardwareWriteCmd::new(
        SVAKOM_V6_VIBRATOR_UUID,
        Endpoint::Tx,
        [
          0x55,
          0x03,
          if (vibe1 > 0 && vibe2 > 0) || vibe1 == vibe2 {
            0x00
          } else if vibe1 > 0 {
            0x01
          } else {
            0x02
          },
          0x00,
          if vibe1 == vibe2 && vibe1 == 0 {
            0x00
          } else {
            0x01
          },
          vibe1.max(vibe2) as u8,
        ]
        .to_vec(),
        false,
      )
      .into()])
    } else {
      let vibe3 = self.last_vibrator_speeds[2].load(Ordering::Relaxed);
      Ok(vec![HardwareWriteCmd::new(
        feature_id,
        Endpoint::Tx,
        [
          0x55,
          0x07,
          0x00,
          0x00,
          if vibe3 == 0 { 0x00 } else { 0x01 },
          vibe3 as u8,
          0x00,
        ]
        .to_vec(),
        false,
      ).into()])
    }
  }
}
