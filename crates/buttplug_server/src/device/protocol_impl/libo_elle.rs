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

generic_protocol_setup!(LiboElle, "libo-elle");

#[derive(Default)]
pub struct LiboElle {}

impl ProtocolHandler for LiboElle {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![{
      let speed = speed as u8;
      if feature_index == 1 {
        let mut data = 0u8;
        if speed > 0 && speed <= 7 {
          data |= (speed - 1) << 4;
          data |= 1; // Set the mode too
        } else if speed > 7 {
          data |= (speed - 8) << 4;
          data |= 4; // Set the mode too
        }
        HardwareWriteCmd::new(&[feature_id], Endpoint::Tx, vec![data], false).into()
      } else {
        HardwareWriteCmd::new(&[feature_id], Endpoint::TxMode, vec![speed], false).into()
      }
    }])
  }
}
