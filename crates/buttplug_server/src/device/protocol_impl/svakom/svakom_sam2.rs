// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler, ProtocolKeepaliveStrategy},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

generic_protocol_setup!(SvakomSam2, "svakom-sam2");

#[derive(Default)]
pub struct SvakomSam2 {}

impl ProtocolHandler for SvakomSam2 {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
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
      [
        0x55,
        0x03,
        0x00,
        0x00,
        if speed == 0 { 0x00 } else { 0x05 },
        speed as u8,
        0x00,
      ]
      .to_vec(),
      true,
    )
    .into()])
  }

  fn handle_output_constrict_cmd(
    &self,
    _feature_index: u32,
    feature_id: uuid::Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      [
        0x55,
        0x09,
        0x00,
        0x00,
        if level == 0 { 0x00 } else { 0x01 },
        level as u8,
        0x00,
      ]
      .to_vec(),
      true,
    )
    .into()])
  }
}
