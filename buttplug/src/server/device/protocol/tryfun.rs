// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
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

generic_protocol_setup!(TryFun, "tryfun");

#[derive(Default)]
pub struct TryFun {}

impl ProtocolHandler for TryFun {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_oscillate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut sum: u8 = 0xff;
    let mut data = vec![0xAA, 0x02, 0x07, scalar as u8];
    let mut count = 0;
    for item in data.iter().skip(1) {
      sum -= item;
      count += 1;
    }
    sum += count;
    data.push(sum);

    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, true).into()])
  }

  fn handle_scalar_rotate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut sum: u8 = 0xff;
    let mut data = vec![0xAA, 0x02, 0x08, scalar as u8];
    let mut count = 0;
    for item in data.iter().skip(1) {
      sum -= item;
      count += 1;
    }
    sum += count;
    data.push(sum);

    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, true).into()])
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        0x00,
        0x02,
        0x00,
        0x05,
        if scalar == 0 { 1u8 } else { 2u8 },
        if scalar == 0 { 2u8 } else { scalar as u8 },
        0x01,
        if scalar == 0 { 1u8 } else { 0u8 },
        0xfd - (scalar as u8).max(1),
      ],
      true,
    )
    .into()])
  }
}
