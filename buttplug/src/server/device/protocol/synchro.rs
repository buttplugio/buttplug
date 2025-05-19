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

generic_protocol_setup!(Synchro, "synchro");

#[derive(Default)]
pub struct Synchro {}

impl ProtocolHandler for Synchro {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_rotation_with_direction_cmd(
    &self,
    cmd: &CheckedValueWithParameterCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      cmd.feature_uuid(),
      Endpoint::Tx,
      vec![
        0xa1,
        0x01,
        cmd.value() as u8
          | if cmd.parameter() > 0 || cmd.value() == 0 {
            0x00
          } else {
            0x80
          },
        0x77,
        0x55,
      ],
      false,
    )
    .into()])
  }
}
