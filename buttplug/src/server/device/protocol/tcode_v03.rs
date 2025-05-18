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
    message::checked_value_with_parameter_cmd::CheckedValueWithParameterCmdV4,
  },
};

generic_protocol_setup!(TCodeV03, "tcode-v03");

#[derive(Default)]
pub struct TCodeV03 {}

impl ProtocolHandler for TCodeV03 {
  fn handle_position_with_duration_cmd(
    &self,
    msg: CheckedValueWithParameterCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut msg_vec = vec![];
    for v in msg.vectors() {
      let position = (v.position() * 99f64) as u32;

      let command = format!("L{}{:02}I{}\n", v.feature_index(), position, v.duration());
      msg_vec.push(HardwareWriteCmd::new(Endpoint::Tx, command.as_bytes().to_vec(), false).into());
    }
    Ok(msg_vec)
  }

  fn handle_value_vibrate_cmd(
    &self,
    index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    debug!("TCODE VIBRATE COMMAND");
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      format!("V{}{:02}\n", index, scalar).as_bytes().to_vec(),
      false,
    )
    .into()])
  }
}
