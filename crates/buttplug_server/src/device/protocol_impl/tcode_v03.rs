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

generic_protocol_setup!(TCodeV03, "tcode-v03");

#[derive(Default)]
pub struct TCodeV03 {}

impl ProtocolHandler for TCodeV03 {
  fn handle_output_position_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    position: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut msg_vec = vec![];

    let command = format!("L0{position:03}\nR0{position:03}\n");
    msg_vec.push(
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        command.as_bytes().to_vec(),
        false,
      )
      .into(),
    );

    Ok(msg_vec)
  }

  fn handle_position_with_duration_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    position: u32,
    duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut msg_vec = vec![];

    let command = format!("L{feature_index}{position:02}I{duration}\n");
    msg_vec.push(
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        command.as_bytes().to_vec(),
        false,
      )
      .into(),
    );

    Ok(msg_vec)
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      format!("V{feature_index}{speed:02}\n").as_bytes().to_vec(),
      false,
    )
    .into()])
  }
}
