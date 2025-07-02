// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::{uuid, Uuid};

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler, ProtocolKeepaliveStrategy},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use std::sync::atomic::{AtomicU8, Ordering};
generic_protocol_setup!(SvakomV5, "svakom-v5");

const SVAKOM_V5_VIBRATOR_UUID: Uuid = uuid!("d19af460-3d81-483b-a87f-b2781d972bac");

#[derive(Default)]
pub struct SvakomV5 {
  last_vibrator_speeds: [AtomicU8; 2],
}

impl ProtocolHandler for SvakomV5 {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.last_vibrator_speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    let vibe1 = self.last_vibrator_speeds[0].load(Ordering::Relaxed);
    let vibe2 = self.last_vibrator_speeds[1].load(Ordering::Relaxed);
    Ok(vec![HardwareWriteCmd::new(
      &[SVAKOM_V5_VIBRATOR_UUID],
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
  }

  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      [0x55, 0x09, 0x00, 0x00, speed as u8, 0x00].to_vec(),
      false,
    )
    .into()])
  }
}
