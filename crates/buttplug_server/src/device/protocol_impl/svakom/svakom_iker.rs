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

// Each motor's packets carry a fixed command id, whichever feature triggered them, so device task
// batching collapses repeats of the same packet but never drops one motor's packet for the other's.
const VIBRATOR_COMMAND_UUID: Uuid = uuid!("0c7e4a2d-9b31-4f68-a5d2-3e8f1b6c9a07");
const PULSATOR_COMMAND_UUID: Uuid = uuid!("5b0f3c8e-2d4a-4e1b-9c6f-8a7d1e2b3c4d");

#[derive(Default)]
pub struct SvakomIker {
  last_speeds: Arc<[AtomicU8; 2]>,
}

// Vibrator speed 0 in mode 1 keeps it running; mode 0 stops it, and the pulsator with it.
fn vibrator_packet(speed: u8) -> HardwareCommand {
  HardwareWriteCmd::new(
    &[VIBRATOR_COMMAND_UUID],
    Endpoint::Tx,
    vec![0x55, 0x03, 0x03, 0x00, (speed > 0) as u8, speed],
    false,
  )
  .into()
}

// Pulsator speed 0 stops only the pulsator.
fn pulsator_packet(speed: u8) -> HardwareCommand {
  HardwareWriteCmd::new(
    &[PULSATOR_COMMAND_UUID],
    Endpoint::Tx,
    vec![0x55, 0x07, 0x00, 0x00, speed, 0x00],
    false,
  )
  .into()
}

impl ProtocolHandler for SvakomIker {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.last_speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    let vibe0 = self.last_speeds[0].load(Ordering::Relaxed);
    let vibe1 = self.last_speeds[1].load(Ordering::Relaxed);
    if vibe0 == 0 && vibe1 == 0 {
      return Ok(vec![vibrator_packet(0)]);
    }
    if feature_index == 1 {
      // Sending the vibrator packet here would stop and restart the pulsator on every pulsator
      // change while the vibrator is off.
      return Ok(vec![pulsator_packet(vibe1)]);
    }
    let mut packets = vec![vibrator_packet(vibe0)];
    if vibe1 > 0 {
      // Restarts the pulsator after a vibrator stop, and reasserts it otherwise.
      packets.push(pulsator_packet(vibe1));
    }
    Ok(packets)
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
