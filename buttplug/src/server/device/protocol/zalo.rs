// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(Zalo, "zalo");

#[derive(Default)]
pub struct Zalo {}

impl ProtocolHandler for Zalo {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // Store off result before the match, so we drop the lock ASAP.
    let speed0: u8 = cmds[0].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8;
    let speed1: u8 = if cmds.len() == 1 {
      0
    } else {
      cmds[1].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8
    };
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        if speed0 == 0 && speed1 == 0 {
          0x02
        } else {
          0x01
        },
        if speed0 == 0 { 0x01 } else { speed0 },
        if speed1 == 0 { 0x01 } else { speed1 },
      ],
      true,
    )
    .into()])
  }
}
