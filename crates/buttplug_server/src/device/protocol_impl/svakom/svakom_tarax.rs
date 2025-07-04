// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use uuid::Uuid;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};

generic_protocol_setup!(SvakomTaraX, "svakom-tarax");

#[derive(Default)]
pub struct SvakomTaraX {}

impl ProtocolHandler for SvakomTaraX {
  // I am like 90% sure this is wrong since this device has two vibrators, but the original
  // implementation made no sense in terms of knowing which command addressed which index. Putting
  // in a best effort here and we'll see if anyone complains.
  fn handle_output_vibrate_cmd(
      &self,
      _feature_index: u32,
      feature_id: Uuid,
      speed: u32,
    ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec!(HardwareWriteCmd::new(
      &[feature_id],
        Endpoint::Tx,
        [
          0x55,
          0x03,
          0x00,
          0x00,
          if speed == 0 { 0x01 } else { speed as u8 },
          if speed == 0 { 0x01 } else { 0x02 },
        ]
        .to_vec(),
        false,
      ).into()))
  }
}
