// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.
use uuid::Uuid;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolKeepaliveStrategy, generic_protocol_setup},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

generic_protocol_setup!(MononiiSkullman, "mononii-skullman");

#[derive(Default)]
pub struct MononiiSkullman {}

impl ProtocolHandler for MononiiSkullman {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
  }

  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        [
          0x55,
          0x03,
          0x00,
          0x00,
          if speed == 0 { 0x00 } else { 0x01 },
          speed as u8,
        ]
        .to_vec(),
        false,
      )
      .into(),
    ])
  }

  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        [0x55, 0x08, 0x00, 0x00, speed as u8, 0x00].to_vec(),
        false,
      )
          .into(),
    ])
  }

  fn handle_output_temperature_cmd(&self, _feature_index: u32, feature_id: Uuid, level: i32) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        if level == 0 { vec![0x55, 0x05, 0x00, 0x00, 0x00, 0x00]} else {
        vec![0x55, 0x05, 0x01, 0x37, 0x00, 0x00] },
        false,
      )
          .into(),
    ])
  }
}
