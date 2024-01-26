// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::ActuatorType;
use crate::core::message::ActuatorType::{Constrict, Oscillate, Rotate, Vibrate};
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(MetaXSire, "metaxsire");

#[derive(Default)]
pub struct MetaXSire {}

impl ProtocolHandler for MetaXSire {
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
    let mut data: Vec<u8> = vec![0x23, 0x07];
    data.push((commands.len() * 3) as u8);

    for (i, item) in commands.iter().enumerate() {
      let cmd = item.unwrap_or((Vibrate, 0));
      // motor number
      data.push(0x80 | ((i + 1) as u8));
      // motor type: 03=vibe 04=pump 06=rotate
      data.push(if cmd.0 == Rotate {
        0x06
      } else if cmd.0 == Constrict || cmd.0 == Oscillate {
        0x04
      } else {
        0x03
      });
      data.push(cmd.1 as u8);
    }

    let mut crc: u8 = 0;
    for b in data.clone() {
      crc ^= b;
    }
    data.push(crc);

    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, false).into()])
  }
}
