// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, messages::{Endpoint, ActuatorType}},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(HtkBm, "htk_bm");

#[derive(Default)]
pub struct HtkBm {}

impl ProtocolHandler for HtkBm {
  fn handle_scalar_cmd(
    &self,
    cmds: &Vec<Option<(ActuatorType, u32)>>
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut cmd_vec = vec![];
    if cmds.len() == 2 {
      let mut data: u8 = 15;
      let left = cmds[0].unwrap_or((ActuatorType::Vibrate, 0)).1;
      let right = cmds[1].unwrap_or((ActuatorType::Vibrate, 0)).1;
      if left != 0 && right != 0 {
        data = 11 // both (normal mode)
      } else if left != 0 {
        data = 12 // left only
      } else if right != 0 {
        data = 13 // right only
      }
      cmd_vec.push(HardwareWriteCmd::new(Endpoint::Tx, vec![data], false).into());
    }
    Ok(cmd_vec)
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use super::HtkBm;
  use crate::{
    core::messages::{ActuatorType, Endpoint},
    server::device::{
      hardware::{HardwareCommand, HardwareWriteCmd},
      protocol::ProtocolHandler,
    },
  };

  #[test]
  pub fn test_htkbm_protocol() {
    let handler = HtkBm {};
    assert_eq!(
      handler.handle_scalar_cmd(&vec![Some((ActuatorType::Vibrate, 0)), Some((ActuatorType::Vibrate, 0))]),
      Ok(vec![HardwareCommand::Write(HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![15],
        false
      ))])
    );
    assert_eq!(
      handler.handle_scalar_cmd(&vec![Some((ActuatorType::Vibrate, 1)), Some((ActuatorType::Vibrate, 0))]),
      Ok(vec![HardwareCommand::Write(HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![12],
        false
      ))])
    );
    assert_eq!(
      handler.handle_scalar_cmd(&vec![Some((ActuatorType::Vibrate, 0)), Some((ActuatorType::Vibrate, 1))]),
      Ok(vec![HardwareCommand::Write(HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![13],
        false
      ))])
    );
    assert_eq!(
      handler.handle_scalar_cmd(&vec![Some((ActuatorType::Vibrate, 1)), Some((ActuatorType::Vibrate, 1))]),
      Ok(vec![HardwareCommand::Write(HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![11],
        false
      ))])
    );
  }
}
