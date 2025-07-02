// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
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

generic_protocol_setup!(SvakomDice, "svakom-dice");

#[derive(Default)]
pub struct SvakomDice {}

impl ProtocolHandler for SvakomDice {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
  }

  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      [0x55, 0x04, 0x00, 0x00, 01, speed as u8, 0xaa].to_vec(),
      false,
    )
    .into()])
  }
}

/*
Start roll sensor: tx: 550e01
Stop roll sensor: tx: 550e00

Started roll: rx: 5508d01000000
Settled roll: rx: 5508d00000000

Battery: rx: 55080000001XX 0-100
*/
