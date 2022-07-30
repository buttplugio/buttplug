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
  fn handle_scalar_vibrate_cmd(
    &self,
    index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![2u8 + (index as u8), 1u8 + (index as u8), scalar as u8, 0u8],
      false,
    )
    .into()])
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use crate::{
    core::messages::{ActuatorType, Endpoint},
    server::device::{
      hardware::{HardwareCommand, HardwareWriteCmd},
      protocol::ProtocolHandler,
    },
  };

  #[test]
  pub fn test_cachito_protocol() {
    let handler = super::Cachito::default();
    assert_eq!(
      handler.handle_scalar_cmd(&vec![Some((ActuatorType::Vibrate, 3))]),
      Ok(vec![HardwareCommand::Write(HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![2, 1, 3, 0],
        false
      ))])
    );
    assert_eq!(
      handler.handle_scalar_cmd(&vec![
        Some((ActuatorType::Vibrate, 1)),
        Some((ActuatorType::Vibrate, 50))
      ]),
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
