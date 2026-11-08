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
use std::sync::atomic::{AtomicU8, Ordering};
use uuid::{Uuid, uuid};

// Tag for the vibrate-restore write emitted by the oscillate handler. It must
// differ from the oscillate feature id, otherwise the device task's per-feature
// dedupe would collapse the two writes into one and drop the master level.
const SVAKOM_JORDAN_VIBRATE_RESTORE_ID: Uuid = uuid!("3f1c6a3e-6f3e-4d7a-9c1b-7b4a2e9d5c01");

generic_protocol_setup!(SvakomJordan, "svakom-jordan");

// Svakom Jordan command layout (sniffed from the official app and verified on
// hardware):
//
//   55 03 00 00 <on> <speed> 00   vibrate level only (0-10)
//   55 04 00 00 00   <level> aa   master level: sets BOTH thrust and vibration
//                                 to <level> (0-255)
//   55 08 00 00 <on> <xx>    00   thrust on/off; the speed byte is ignored by
//                                 the firmware, so thrust only runs at one speed
//
// There is no thrust-only speed command. However, a vibrate command sent after
// the master level overrides the vibration level while thrust keeps running at
// the master level. So to set thrust speed we send the master level, then
// immediately restore the last requested vibration level. Master level 0 does
// not reliably stop thrust when a vibrate frame follows it, so thrust off uses
// the dedicated 0x08 off frame, which leaves vibration untouched.
#[derive(Default)]
pub struct SvakomJordan {
  last_vibrate: AtomicU8,
}

impl SvakomJordan {
  fn vibrate_cmd(command_id: Uuid, speed: u8) -> HardwareCommand {
    HardwareWriteCmd::new(
      &[command_id],
      Endpoint::Tx,
      vec![
        0x55,
        0x03,
        0x00,
        0x00,
        if speed == 0 { 0x00 } else { 0x01 },
        speed,
        0x00,
      ],
      false,
    )
    .into()
  }
}

impl ProtocolHandler for SvakomJordan {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
  }

  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.last_vibrate.store(speed as u8, Ordering::Relaxed);
    Ok(vec![Self::vibrate_cmd(feature_id, speed as u8)])
  }

  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if speed == 0 {
      return Ok(vec![
        HardwareWriteCmd::new(
          &[feature_id],
          Endpoint::Tx,
          vec![0x55, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00],
          false,
        )
        .into(),
      ]);
    }
    Ok(vec![
      // Master level: drives thrust (and vibration) at `speed`.
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0x55, 0x04, 0x00, 0x00, 0x00, speed as u8, 0xaa],
        false,
      )
      .into(),
      // Put vibration back to whatever the user last asked for.
      Self::vibrate_cmd(
        SVAKOM_JORDAN_VIBRATE_RESTORE_ID,
        self.last_vibrate.load(Ordering::Relaxed),
      ),
    ])
  }
}
