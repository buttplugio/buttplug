// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::Uuid;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

generic_protocol_setup!(IToys, "itoys");

#[derive(Default)]
pub struct IToys {}

impl ProtocolHandler for IToys {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![0xa0, 0x01, 0x00, 0x00, speed as u8, 0xff],
      false,
    )
    .into()])
  }


    fn handle_output_oscillate_cmd(&self, _feature_index: u32, feature_id: Uuid, speed: u32) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
        Ok(vec![HardwareWriteCmd::new(
            &[feature_id],
            Endpoint::Tx,
            vec![
                0xa0,
                0x06,
                if speed == 0 { 0x00 } else { 0x01 },
                0x00,
                if speed == 0 { 0x00 } else { 0x01 },
                speed as u8,
            ],
            false,
        )
            .into()])
    }
}
