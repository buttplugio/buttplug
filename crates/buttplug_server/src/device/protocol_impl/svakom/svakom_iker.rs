// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler, ProtocolKeepaliveStrategy},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

generic_protocol_setup!(SvakomIker, "svakom-iker");

#[derive(Default)]
pub struct SvakomIker {
  last_speeds: Arc<[AtomicU8; 2]>,
}

impl ProtocolHandler for SvakomIker {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.last_speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    let vibe0 = self.last_speeds[0].load(Ordering::Relaxed);
    let vibe1 = self.last_speeds[1].load(Ordering::Relaxed);
    if vibe0 == 0 && vibe1 == 0 {
      Ok(vec![HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        [0x55, 0x07, 0x00, 0x00, 0x00, 0x00].to_vec(),
        false,
      )
      .into()])
    } else {
      let mut msgs = vec![];
      msgs.push(
        HardwareWriteCmd::new(
          &[feature_id],
          Endpoint::Tx,
          [0x55, 0x03, 0x03, 0x00, 0x01, vibe0 as u8].to_vec(),
          false,
        )
        .into(),
      );
      if vibe1 > 0 {
        msgs.push(
          HardwareWriteCmd::new(
            &[feature_id],
            Endpoint::Tx,
            [0x55, 0x07, 0x00, 0x00, vibe1 as u8, 0x00].to_vec(),
            false,
          )
          .into(),
        );
      }
      Ok(msgs)
    }
  }
}
