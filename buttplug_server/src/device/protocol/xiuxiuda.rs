// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::Uuid;

use buttplug_core::{errors::ButtplugDeviceError, message::Endpoint};
use crate::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
};

generic_protocol_setup!(Xiuxiuda, "xiuxiuda");

#[derive(Default)]
pub struct Xiuxiuda {}

impl ProtocolHandler for Xiuxiuda {


  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      [0x00, 0x00, 0x00, 0x00, 0x65, 0x3a, 0x30, speed as u8, 0x64].to_vec(),
      false,
    )
    .into()])
  }
}
