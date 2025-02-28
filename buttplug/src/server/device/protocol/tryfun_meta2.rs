// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  generic_protocol_setup,
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::ProtocolHandler,
  },
};
use std::sync::atomic::{AtomicU8, Ordering};

generic_protocol_setup!(TryFunMeta2, "tryfun-meta2");

#[derive(Default)]
pub struct TryFunMeta2 {
  packet_id: AtomicU8,
}

impl ProtocolHandler for TryFunMeta2 {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_oscillate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut sum: u8 = 0xff;
    let mut data = vec![
      self.packet_id.fetch_add(1, Ordering::Relaxed),
      0x02,
      0x00,
      0x05,
      0x21,
      0x05,
      0x0b,
      scalar as u8,
    ];
    let mut count = 1;
    for item in data.iter().skip(1) {
      sum -= item;
      count += 1;
    }
    sum += count;
    data.push(sum);

    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, false).into()])
  }

  fn handle_rotate_cmd(
    &self,
    commands: &[Option<(u32, bool)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if commands.len() >= 1 {
      if let Some(cmd) = commands[0] {
        let mut speed = cmd.0 as i8;
        if cmd.1 {
          speed += 1;
          speed *= -1;
        }
        let mut sum: u8 = 0xff;
        let mut data = vec![
          self.packet_id.fetch_add(1, Ordering::Relaxed),
          0x02,
          0x00,
          0x05,
          0x21,
          0x05,
          0x0e,
          speed as u8,
        ];
        let mut count = 1;
        for item in data.iter().skip(1) {
          sum -= item;
          count += 1;
        }
        sum += count;
        data.push(sum);

        return Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, false).into()]);
      }
    }
    Ok(vec![])
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut sum: u8 = 0xff;
    let mut data = vec![
      self.packet_id.fetch_add(1, Ordering::Relaxed),
      0x02,
      0x00,
      0x05,
      0x21,
      0x05,
      0x08,
      scalar as u8,
    ];
    let mut count = 1;
    for item in data.iter().skip(1) {
      sum -= item;
      count += 1;
    }
    sum += count;
    data.push(sum);

    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, false).into()])
  }
}
