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

generic_protocol_setup!(Aneros, "aneros");

#[derive(Default)]
pub struct Aneros {}

impl ProtocolHandler for Aneros {
  fn handle_scalar_vibrate_cmd(
    &self,
    index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(
      vec![HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![0xF1 + (index as u8), scalar as u8],
        false,
      ).into()]
    )
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use super::Aneros;
  use crate::{
    core::messages::{ActuatorType, Endpoint},
    server::device::{
      hardware::{HardwareCommand, HardwareWriteCmd},
      protocol::ProtocolHandler,
    },
  };

  #[test]
  pub fn test_aneros_protocol() {
    let handler = Aneros {};
    assert_eq!(
      handler.handle_scalar_cmd(&vec![Some((ActuatorType::Vibrate, 64))]),
      Ok(vec![HardwareCommand::Write(HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![0xF1, 64],
        false
      ))])
    );
    assert_eq!(
      handler.handle_scalar_cmd(&vec![Some((ActuatorType::Vibrate, 13)), Some((ActuatorType::Vibrate, 64))]),
      Ok(vec![
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF1, 13], false)),
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF2, 64], false))
      ])
    );
  }
}
