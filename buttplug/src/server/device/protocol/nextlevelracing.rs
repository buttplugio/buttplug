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
  }, message::checked_actuator_cmd::CheckedActuatorCmdV4},
};

generic_protocol_setup!(NextLevelRacing, "nextlevelracing");

#[derive(Default)]
pub struct NextLevelRacing {}

impl ProtocolHandler for NextLevelRacing {
  fn handle_value_vibrate_cmd(
    &self,
    cmd: &CheckedActuatorCmdV4
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      cmd.feature_id(),
      Endpoint::Tx,
      format!("M{}{}\r", cmd.feature_index(), cmd.value()).into_bytes(),
      false,
    )
    .into()])
  }
}
