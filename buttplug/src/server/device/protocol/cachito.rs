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
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(Cachito, "cachito");

#[derive(Default)]
pub struct Cachito {}

impl ProtocolHandler for Cachito {
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
            vec![2u8 + (index as u8), 1u8 + (index as u8), *speed as u8, 0u8],
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
  use crate::{
    core::messages::Endpoint,
    server::device::{
      hardware::{HardwareCommand, HardwareWriteCmd},
      protocol::ProtocolHandler,
    },
  };

  #[test]
  pub fn test_cachito_protocol() {
    let handler = super::Cachito::default();
    assert_eq!(
      handler.handle_vibrate_cmd(&vec![Some(3)]),
      Ok(vec![HardwareCommand::Write(HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![2, 1, 3, 0],
        false
      ))])
    );
    assert_eq!(
      handler.handle_vibrate_cmd(&vec![Some(1), Some(50)]),
      Ok(vec![
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![2, 1, 1, 0], false)),
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![3, 2, 50, 0],
          false
        )),
      ])
    );
  }
}
