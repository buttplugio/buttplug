// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};

generic_protocol_setup!(TryFun, "tryfun");

#[derive(Default)]
pub struct TryFun {}

impl ProtocolHandler for TryFun {
  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut sum: u8 = 0xff;
    let mut data = vec![0xAA, 0x02, 0x07, speed as u8];
    let mut count = 0;
    for item in data.iter().skip(1) {
      sum -= item;
      count += 1;
    }
    sum += count;
    data.push(sum);

    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      data,
      true,
    )
    .into()])
  }

  fn handle_output_rotate_cmd(
    &self,
    _feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut sum: u8 = 0xff;
    let mut data = vec![0xAA, 0x02, 0x08, speed as u8];
    let mut count = 0;
    for item in data.iter().skip(1) {
      sum -= item;
      count += 1;
    }
    sum += count;
    data.push(sum);

    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      data,
      true,
    )
    .into()])
  }

  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![
        0x00,
        0x02,
        0x00,
        0x05,
        if speed == 0 { 1u8 } else { 2u8 },
        if speed == 0 { 2u8 } else { speed as u8 },
        0x01,
        if speed == 0 { 1u8 } else { 0u8 },
        0xfd - (speed as u8).max(1),
      ],
      true,
    )
    .into()])
  }
}
