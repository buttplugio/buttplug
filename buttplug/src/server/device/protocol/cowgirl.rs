// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::errors::ButtplugDeviceError::ProtocolSpecificError;
use crate::core::message::ActuatorType;
use crate::core::message::ActuatorType::{Rotate, Vibrate};
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(Cowgirl, "cowgirl");

#[derive(Default)]
pub struct Cowgirl {}

impl ProtocolHandler for Cowgirl {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn handle_scalar_cmd(
    &self,
    commands: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut data: Vec<u8> = vec![0x00, 0x01];
    if commands.len() != 2 {
      return Err(ProtocolSpecificError(
        "cowgirl".to_owned(),
        format!("Expected 2 attributes, got {}", commands.len()),
      ));
    }

    if let Some(cmd) = commands[0] {
      if cmd.0 != Vibrate {
        return Err(ProtocolSpecificError(
          "cowgirl".to_owned(),
          format!("Expected Vibrate attribute, got {:?}", cmd.0),
        ));
      }
      data.push(cmd.1 as u8);
    } else {
      return Err(ProtocolSpecificError(
        "cowgirl".to_owned(),
        "Attribute 0 is None".to_owned(),
      ));
    }

    if let Some(cmd) = commands[1] {
      if cmd.0 != Rotate {
        return Err(ProtocolSpecificError(
          "cowgirl".to_owned(),
          format!("Expected Rotate attribute, got {:?}", cmd.0),
        ));
      }
      data.push(cmd.1 as u8);
    } else {
      return Err(ProtocolSpecificError(
        "cowgirl".to_owned(),
        "Attribute 1 is None".to_owned(),
      ));
    }

    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, true).into()])
  }
}
