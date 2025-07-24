// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler, ProtocolKeepaliveStrategy},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use std::time::Duration;
use uuid::Uuid;

generic_protocol_setup!(SexverseV3, "sexverse-v3");

const SEXVERSE_COMMAND_DELAY_MS: u64 = 100;

#[derive(Default)]
pub struct SexverseV3 {}

impl SexverseV3 {
  fn form_command(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![0xa1, 0x04, speed as u8, feature_index as u8 + 1],
      true,
    )
    .into()])
  }
}

impl ProtocolHandler for SexverseV3 {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(Duration::from_millis(
      SEXVERSE_COMMAND_DELAY_MS,
    ))
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_command(feature_index, feature_id, speed)
  }

  fn handle_output_rotate_cmd(
    &self,
    feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_command(feature_index, feature_id, speed)
  }
}
