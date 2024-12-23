// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(FleshyThrust, "fleshy-thrust");

#[derive(Default)]
pub struct FleshyThrust {}

impl ProtocolHandler for FleshyThrust {
  fn handle_linear_cmd(
    &self,
    message: crate::core::message::LinearCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let current_cmd = message
      .vectors()
      .first()
      .ok_or(ButtplugDeviceError::DeviceFeatureCountMismatch(1, 0))?;

    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        (current_cmd.position() * 180f64).abs() as u8,
        ((current_cmd.duration() & 0xff00) >> 8) as u8,
        (current_cmd.duration() & 0xff) as u8,
      ],
      false,
    )
    .into()])
  }
}
