// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint}, server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{
      vorze_sa::VorzeDevice, ProtocolHandler
    },
  }
};
use std::sync::atomic::{AtomicI8, Ordering};

pub struct VorzeSAUfo {
  device_type: VorzeDevice,
  speeds: [AtomicI8; 2]
}

impl VorzeSAUfo {
  pub fn new(device_type: VorzeDevice) -> Self {
    Self {
      device_type,
      speeds: [AtomicI8::new(0), AtomicI8::new(0)]
    }
  }
}

impl ProtocolHandler for VorzeSAUfo {

  fn handle_rotation_with_direction_cmd(
      &self,
      feature_index: u32,
      feature_id: uuid::Uuid,
      speed: u32,
      clockwise: bool,
    ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(if clockwise { speed as i8 } else { -(speed as i8) }, Ordering::Relaxed);
    let speed_left = self.speeds[0].load(Ordering::Relaxed);
    let data_left = ((speed_left >= 0) as u8) << 7 | (speed_left.unsigned_abs());
    let speed_right = self.speeds[1].load(Ordering::Relaxed);
    let data_right = ((speed_right >= 0) as u8) << 7 | (speed_right.unsigned_abs());
    Ok(vec![HardwareWriteCmd::new(
      feature_id,
      Endpoint::Tx,
      vec![self.device_type as u8, data_left, data_right],
      true,
    )
    .into()])
  }

}
