// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::{Uuid, uuid};

use super::VorzeDevice;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::ProtocolHandler,
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use std::sync::atomic::{AtomicI8, Ordering};

// Vorze UFO needs a unified protocol UUID since we update both outputs in the same packet.
const VORZE_UFO_PROTOCOL_UUID: Uuid = uuid!("013c2d1f-b3c0-4372-9cf6-e5fafd3b7631");

#[derive(Default)]
pub struct VorzeSADualRotator {
  speeds: [AtomicI8; 2],
}

impl ProtocolHandler for VorzeSADualRotator {
  fn handle_output_rotate_cmd(
      &self,
      feature_index: u32,
      _feature_id: Uuid,
      speed: i32,
    ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as i8, Ordering::Relaxed);
    let speed_left = self.speeds[0].load(Ordering::Relaxed);
    let data_left = ((speed_left >= 0) as u8) << 7 | (speed_left.unsigned_abs());
    let speed_right = self.speeds[1].load(Ordering::Relaxed);
    let data_right = ((speed_right >= 0) as u8) << 7 | (speed_right.unsigned_abs());
    Ok(vec![
      HardwareWriteCmd::new(
        &[VORZE_UFO_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![VorzeDevice::UfoTw as u8, data_left, data_right],
        true,
      )
      .into(),
    ])
  }
}
