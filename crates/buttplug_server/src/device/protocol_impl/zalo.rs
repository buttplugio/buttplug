// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::atomic::{AtomicU8, Ordering};

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

generic_protocol_setup!(Zalo, "zalo");

#[derive(Default)]
pub struct Zalo {
  speeds: [AtomicU8; 2],
}

impl ProtocolHandler for Zalo {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    let speed0: u8 = self.speeds[0].load(Ordering::Relaxed);
    let speed1: u8 = self.speeds[1].load(Ordering::Relaxed);
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![
        if speed0 == 0 && speed1 == 0 {
          0x02
        } else {
          0x01
        },
        if speed0 == 0 { 0x01 } else { speed0 },
        if speed1 == 0 { 0x01 } else { speed1 },
      ],
      true,
    )
    .into()])
  }
}
