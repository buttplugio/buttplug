// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::{device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  }, message::{checked_actuator_cmd::CheckedActuatorCmdV4, checked_value_with_parameter_cmd::CheckedValueWithParameterCmdV4}},
};

generic_protocol_setup!(NexusRevo, "nexus-revo");

#[derive(Default)]
pub struct NexusRevo {}

impl ProtocolHandler for NexusRevo {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_value_vibrate_cmd(
    &self,
    cmd: &CheckedActuatorCmdV4
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      cmd.feature_id(),
      Endpoint::Tx,
      vec![0xaa, 0x01, 0x01, 0x00, 0x01, cmd.value() as u8],
      true,
    )
    .into()])
  }

  fn handle_rotation_with_direction_cmd(
      &self,
      cmd: &CheckedValueWithParameterCmdV4,
    ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      cmd.feature_id(),
      Endpoint::Tx,
      vec![
        0xaa,
        0x01,
        0x02,
        0x00,
        cmd.value() as u8 + if cmd.value() != 0 && cmd.parameter() > 0 { 2 } else { 0 },
        0x00,
      ],
      true,
      )
      .into()])
  }
}
