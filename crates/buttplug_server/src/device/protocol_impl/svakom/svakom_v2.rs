// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::Uuid;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler, ProtocolKeepaliveStrategy},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

generic_protocol_setup!(SvakomV2, "svakom-v2");

#[derive(Default)]
pub struct SvakomV2 {}

impl ProtocolHandler for SvakomV2 {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if feature_index == 1 {
      Ok(vec![HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        [0x55, 0x06, 0x01, 0x00, speed as u8, speed as u8].to_vec(),
        true,
      )
      .into()])
    } else {
      Ok(vec![HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        [
          0x55,
          0x03,
          0x03,
          0x00,
          if speed == 0 { 0x00 } else { 0x01 },
          speed as u8,
        ]
        .to_vec(),
        true,
      )
      .into()])
    }
  }
}
