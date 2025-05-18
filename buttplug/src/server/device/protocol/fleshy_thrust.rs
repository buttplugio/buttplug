// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::{device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  }, message::checked_value_with_parameter_cmd::CheckedValueWithParameterCmdV4},
};

generic_protocol_setup!(FleshyThrust, "fleshy-thrust");

#[derive(Default)]
pub struct FleshyThrust {}

impl ProtocolHandler for FleshyThrust {
  fn handle_position_with_duration_cmd(
    &self,
    message: &CheckedValueWithParameterCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        message.value() as u8,
        ((message.parameter() & 0xff00) >> 8) as u8,
        (message.parameter() & 0xff) as u8,
      ],
      false,
    )
    .into()])
  }
}
