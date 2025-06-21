// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::Endpoint,
  }, generic_protocol_setup, server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{
      ProtocolHandler,
      ProtocolKeepaliveStrategy,
    },
  }
};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

generic_protocol_setup!(SvakomIker, "svakom-iker");


#[derive(Default)]
pub struct SvakomIker {
  last_speeds: Arc<[AtomicU8; 2]>,
}

impl ProtocolHandler for SvakomIker {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_output_vibrate_cmd(
      &self,
      feature_index: u32,
      feature_id: uuid::Uuid,
      speed: u32,
    ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if feature_index == 0 {
      Ok(vec![HardwareWriteCmd::new(
        feature_id,
        Endpoint::Tx,
        [0x55, 0x03, 0x03, 0x00, 0x01, speed as u8].to_vec(),
        false,
      )
      .into()])
    } else {
      Ok(vec![HardwareWriteCmd::new(
        feature_id,
        Endpoint::Tx,
        [0x55, 0x07, 0x07, 0x00, 0x01, speed as u8].to_vec(),
        false,
      )
      .into()])
    }
    /*
    self.last_speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    let mut msg_vec = vec![];
    let speed0 = self.last_speeds[0].load(Ordering::Relaxed);
    let speed1 = self.last_speeds[1].load(Ordering::Relaxed);
    let vibe_off = speed0 == 0 && speed1 == 0;

    if let Some((_, speed)) = cmds[0] {
      self.last_speeds[0].store(speed as u8, Ordering::Relaxed);
      if speed == 0 {
        vibe_off = true;
      }
      msg_vec.push(
        HardwareWriteCmd::new(
          Endpoint::Tx,
          [0x55, 0x03, 0x03, 0x00, 0x01, speed as u8].to_vec(),
          false,
        )
        .into(),
      );
    }
    if cmds.len() > 1 {
      if let Some((_, speed)) = cmds[1] {
        self.last_speeds[1].store(speed as u8, Ordering::Relaxed);
        msg_vec.push(
          HardwareWriteCmd::new(
            Endpoint::Tx,
            [0x55, 0x07, 0x00, 0x00, speed as u8, 0x00].to_vec(),
            false,
          )
          .into(),
        );
      } else if vibe_off && self.last_speeds[1].load(Ordering::Relaxed) != 0 {
        msg_vec.push(
          HardwareWriteCmd::new(
            Endpoint::Tx,
            [
              0x55,
              0x07,
              0x00,
              0x00,
              self.last_speeds[1].load(Ordering::Relaxed),
              0x00,
            ]
            .to_vec(),
            false,
          )
          .into(),
        );
      }
    }
    Ok(msg_vec)
    */
  }
}
