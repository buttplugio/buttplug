// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler, ProtocolKeepaliveStrategy},
  },
};

generic_protocol_setup!(SvakomBarnard, "svakom-barnard");

#[derive(Default)]
pub struct SvakomBarnard {}

impl ProtocolHandler for SvakomBarnard {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_output_vibrate_cmd(
      &self,
      _feature_index: u32,
      feature_id: uuid::Uuid,
      speed: u32,
    ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      feature_id,
      Endpoint::Tx,
      [
        0x55,
        0x03,
        0x00,
        0x00,
        speed as u8,
        if speed == 0 { 0x00 } else { 0x01 },
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
      feature_id,
      Endpoint::Tx,
      [
        0x55,
        0x08,
        0x00,
        0x00,
        speed as u8,
        if speed == 0 { 0x00 } else { 0xff },
      ]
      .to_vec(),
      false,
    )
    .into()])
  }
}
