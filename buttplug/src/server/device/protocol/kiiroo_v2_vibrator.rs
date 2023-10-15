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

generic_protocol_setup!(KiirooV2Vibrator, "kiiroo-v2-vibrator");

#[derive(Default)]
pub struct KiirooV2Vibrator {}

impl ProtocolHandler for KiirooV2Vibrator {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        cmds
          .get(0)
          .unwrap_or(&None)
          .unwrap_or((ActuatorType::Vibrate, 0))
          .1 as u8,
        cmds
          .get(1)
          .unwrap_or(&None)
          .unwrap_or((ActuatorType::Vibrate, 0))
          .1 as u8,
        cmds
          .get(2)
          .unwrap_or(&None)
          .unwrap_or((ActuatorType::Vibrate, 0))
          .1 as u8,
      ],
      false,
    )
    .into()])
  }
}
