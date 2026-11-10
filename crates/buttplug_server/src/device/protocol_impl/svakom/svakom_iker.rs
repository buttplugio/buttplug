// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolKeepaliveStrategy, generic_protocol_setup},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use uuid::{Uuid, uuid};

generic_protocol_setup!(SvakomIker, "svakom-iker");

// Pulsator packets get their own command id so device task batching never collapses them into the
// vibrator packet sent alongside.
const PULSATOR_COMMAND_UUID: Uuid = uuid!("5b0f3c8e-2d4a-4e1b-9c6f-8a7d1e2b3c4d");

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
    // Vibrator speed 0 in mode 1 keeps it running; mode 0 stops it, and the pulsator with it.
    // Pulsator speed 0 stops only the pulsator.
    if vibe0 == 0 && vibe1 == 0 {
      Ok(vec![
        HardwareWriteCmd::new(
          &[feature_id],
          Endpoint::Tx,
          [0x55, 0x03, 0x03, 0x00, 0x00, 0x00].to_vec(),
          false,
        )
        .into(),
      ])
    } else {
      let mut msgs = vec![];
      msgs.push(
        HardwareWriteCmd::new(
          &[feature_id],
          Endpoint::Tx,
          [0x55, 0x03, 0x03, 0x00, (vibe0 > 0) as u8, vibe0].to_vec(),
          false,
        )
        .into(),
      );
      if vibe1 > 0 || feature_index == 1 {
        msgs.push(
          HardwareWriteCmd::new(
            &[PULSATOR_COMMAND_UUID],
            Endpoint::Tx,
            [0x55, 0x07, 0x00, 0x00, { vibe1 }, 0x00].to_vec(),
            false,
          )
          .into(),
        );
      }
      Ok(msgs)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::SvakomIker;
  use crate::device::{hardware::HardwareCommand, protocol::ProtocolHandler};
  use uuid::Uuid;

  #[test]
  fn vibrator_stop_survives_batching_while_pulsator_runs() {
    let iker = SvakomIker::default();
    let (vibrator, pulsator) = (Uuid::new_v4(), Uuid::new_v4());
    iker.handle_output_vibrate_cmd(1, pulsator, 3).unwrap();
    // The device task dedupes each command of a later batch against what is pending, so writes
    // that share a command id collapse to the last one.
    let mut pending: Vec<HardwareCommand> = vec![];
    for cmd in iker.handle_output_vibrate_cmd(0, vibrator, 0).unwrap() {
      pending.retain(|existing| !cmd.overlaps(existing));
      pending.push(cmd);
    }
    let data: Vec<Vec<u8>> = pending
      .iter()
      .map(|cmd| match cmd {
        HardwareCommand::Write(write) => write.data().clone(),
        _ => panic!("Expected a write command"),
      })
      .collect();
    assert_eq!(
      data,
      vec![
        vec![0x55, 0x03, 0x03, 0x00, 0x00, 0x00],
        vec![0x55, 0x07, 0x00, 0x00, 0x03, 0x00]
      ]
    );
  }
}
