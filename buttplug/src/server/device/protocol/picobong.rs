// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::Uuid;

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(Picobong, "picobong");

#[derive(Default)]
pub struct Picobong {}

impl ProtocolHandler for Picobong {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mode: u8 = if speed == 0 { 0xff } else { 0x01 };
    Ok(vec![HardwareWriteCmd::new(
      feature_id,
      Endpoint::Tx,
      [0x01, mode, speed as u8].to_vec(),
      false,
    )
    .into()])
  }
}

// TODO Write tests for protocol
