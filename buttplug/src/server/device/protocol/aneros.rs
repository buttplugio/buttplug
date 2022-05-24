// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, messages::Endpoint},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{ProtocolHandler, generic_protocol_setup}
  },
};

generic_protocol_setup!(Aneros, "aneros");

#[derive(Default)]
pub struct Aneros {}

impl ProtocolHandler for Aneros {
  fn handle_vibrate_cmd(
    &self,
    cmds: &Vec<Option<u32>>,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut cmd_vec = vec![];
    for (index, cmd) in cmds.iter().enumerate() {
      if let Some(speed) = cmd {
        cmd_vec.push(
          HardwareWriteCmd::new(
            Endpoint::Tx,
            vec![0xF1 + (index as u8), *speed as u8],
            false,
          )
          .into(),
        );
      }
    }
    Ok(cmd_vec)
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use super::Aneros;
  use crate::{
    core::messages::Endpoint,
    server::device::{
      hardware::{HardwareCommand, HardwareWriteCmd},
      protocol::ProtocolHandler,
    }
  };

  #[test]
  pub fn test_aneros_protocol() {
    let handler = Aneros {};
    assert_eq!(handler.handle_vibrate_cmd(&vec![Some(64)]), Ok(vec![HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF1, 64], false))]));
    assert_eq!(
      handler.handle_vibrate_cmd(&vec![Some(13), Some(64)]), 
      Ok(vec![HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF1, 13], false)), HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF2, 64], false))])
    );
  }
}