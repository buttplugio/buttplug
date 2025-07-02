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
use std::time::Duration;
use uuid::Uuid;

generic_protocol_setup!(MizzZeeV3, "mizzzee-v3");

// Time between MizzZee v3 update commands, in milliseconds.
const MIZZZEE3_COMMAND_DELAY_MS: u64 = 200;

fn handle_scale(scale: f32) -> f32 {
  if scale == 0.0 {
    return 0.0;
  }
  scale * 0.7 + 0.3
}

fn scalar_to_vector(scalar: u32) -> Vec<u8> {
  if scalar == 0 {
    return vec![
      0x03, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00,
    ];
  }

  const HEADER: [u8; 3] = [0x03, 0x12, 0xf3];
  const FILL_VEC: [u8; 6] = [0x00, 0xfc, 0x00, 0xfe, 0x40, 0x01];

  let scale: f32 = handle_scale(scalar as f32 / 1000.0) * 1023.0;
  let modded_scale: u16 = ((scale as u16) << 6) | 60;

  let bytes = modded_scale.to_le_bytes();

  let mut data: Vec<u8> = Vec::new();
  data.extend_from_slice(&HEADER);
  data.extend_from_slice(&FILL_VEC);
  data.extend_from_slice(&bytes);
  data.extend_from_slice(&FILL_VEC);
  data.extend_from_slice(&bytes);
  data.push(0x00);

  data
}

#[derive(Default)]
pub struct MizzZeeV3 {}

impl ProtocolHandler for MizzZeeV3 {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(Duration::from_millis(
      MIZZZEE3_COMMAND_DELAY_MS,
    ))
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
      scalar_to_vector(speed),
      true,
    )
    .into()])
  }
}
