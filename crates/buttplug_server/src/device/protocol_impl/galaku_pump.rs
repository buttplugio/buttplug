// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::{uuid, Uuid};

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use std::num::Wrapping;
use std::sync::atomic::{AtomicU8, Ordering};

static KEY_TAB: [[u8; 12]; 4] = [
  [0, 24, 0x98, 0xf7, 0xa5, 61, 13, 41, 37, 80, 68, 70],
  [0, 69, 110, 106, 111, 120, 32, 83, 45, 49, 46, 55],
  [0, 101, 120, 32, 84, 111, 121, 115, 10, 0x8e, 0x9d, 0xa3],
  [0, 0xc5, 0xd6, 0xe7, 0xf8, 10, 50, 32, 111, 98, 13, 10],
];

const GALAKU_PUMP_PROTOCOL_UUID: Uuid = uuid!("165ae3a9-33be-46a8-b438-9a6fc0f183cb");
generic_protocol_setup!(GalakuPump, "galaku-pump");

pub struct GalakuPump {
  speeds: [AtomicU8; 2],
}

impl Default for GalakuPump {
  fn default() -> Self {
    Self {
      speeds: [AtomicU8::new(0), AtomicU8::new(0)],
    }
  }
}

impl GalakuPump {
  fn hardware_command(&self) -> Vec<HardwareCommand> {
    let mut data: Vec<u8> = vec![
      0x23,
      0x5a,
      0x00,
      0x00,
      0x01,
      0x60,
      0x03,
      self.speeds[0].load(Ordering::Relaxed),
      self.speeds[1].load(Ordering::Relaxed),
      0x00,
      0x00,
    ];
    data.push(data.iter().fold(0u8, |c, b| (Wrapping(c) + Wrapping(*b)).0));

    let mut data2: Vec<u8> = vec![0x23];
    for i in 1..data.len() {
      let k = KEY_TAB[(data2[i - 1] & 3) as usize][i];
      data2.push((Wrapping((k ^ 0x23) ^ data[i]) + Wrapping(k)).0);
    }

    vec![HardwareWriteCmd::new(&[GALAKU_PUMP_PROTOCOL_UUID], Endpoint::Tx, data2, true).into()]
  }
}

impl ProtocolHandler for GalakuPump {
  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[0].store(speed as u8, Ordering::Relaxed);
    Ok(self.hardware_command())
  }

  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[1].store(speed as u8, Ordering::Relaxed);
    Ok(self.hardware_command())
  }
}
