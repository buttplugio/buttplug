// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::{
    device::{
      hardware::{HardwareCommand, HardwareWriteCmd},
      protocol::{generic_protocol_setup, ProtocolHandler},
    },
    message::{checked_value_cmd::CheckedValueCmdV4, checked_value_with_parameter_cmd::CheckedValueWithParameterCmdV4},
  },
};

generic_protocol_setup!(TCodeV03, "tcode-v03");

#[derive(Default)]
pub struct TCodeV03 {}

impl ProtocolHandler for TCodeV03 {
  fn handle_position_with_duration_cmd(
    &self,
    cmd: &CheckedValueWithParameterCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut msg_vec = vec![];
    let position = cmd.value() as u32;

    let command = format!("L{}{:02}I{}\n", cmd.feature_index(), position, cmd.parameter() as u32);
    msg_vec.push(HardwareWriteCmd::new(cmd.feature_id(), Endpoint::Tx, command.as_bytes().to_vec(), false).into());

    Ok(msg_vec)
  }

    fn handle_value_vibrate_cmd(
    &self,
    cmd: &CheckedValueCmdV4
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      cmd.feature_id(),
      Endpoint::Tx,
      format!("V{}{:02}\n", cmd.feature_index(), cmd.value()).as_bytes().to_vec(),
      false,
    )
    .into()])
  }
}
