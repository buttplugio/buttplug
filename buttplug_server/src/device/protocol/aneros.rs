// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::Uuid;

use buttplug_core::{errors::ButtplugDeviceError, message::Endpoint};
use crate::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
};

generic_protocol_setup!(Aneros, "aneros");

#[derive(Default)]
pub struct Aneros {}

impl ProtocolHandler for Aneros {


  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![0xF1 + (feature_index as u8), speed as u8],
      false,
    )
    .into()])
  }
}
