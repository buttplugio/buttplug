// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::Uuid;

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::
    device::{
      hardware::{HardwareCommand, HardwareWriteCmd},
      protocol::{generic_protocol_setup, ProtocolHandler},
    },
};

generic_protocol_setup!(TCodeV03, "tcode-v03");

#[derive(Default)]
pub struct TCodeV03 {}

impl ProtocolHandler for TCodeV03 {

  fn handle_actuator_position_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    position: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut msg_vec = vec![];

    let command = format!("L0{:02}\n", position);
    msg_vec.push(HardwareWriteCmd::new(feature_id, Endpoint::Tx, command.as_bytes().to_vec(), false).into());

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
    
    let command = format!("L{}{:02}I{}\n", feature_index, position, duration);
    msg_vec.push(HardwareWriteCmd::new(feature_id, Endpoint::Tx, command.as_bytes().to_vec(), false).into());

    Ok(msg_vec)
  }

    fn handle_actuator_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      feature_id,
      Endpoint::Tx,
      format!("V{}{:02}\n", feature_index, speed).as_bytes().to_vec(),
      false,
    )
    .into()])
  }
}
