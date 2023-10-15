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

generic_protocol_setup!(LovehoneyDesire, "lovehoney-desire");

#[derive(Default)]
pub struct LovehoneyDesire {}

impl ProtocolHandler for LovehoneyDesire {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // The Lovehoney Desire has 2 types of commands
    //
    // - Set both motors with one command
    // - Set each motor separately
    //
    // We'll need to check what we got back and write our
    // commands accordingly.
    //
    // Neat way of checking if everything is the same via
    // https://sts10.github.io/2019/06/06/is-all-equal-function.html.
    //
    // Just make sure we're not matching on None, 'cause if
    // that's the case we ain't got shit to do.
    let mut msg_vec = vec![];
    if cmds[0].is_some() && cmds.windows(2).all(|w| w[0] == w[1]) {
      msg_vec.push(
        HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![
            0xF3,
            0,
            cmds[0].expect("Already checked value existence").1 as u8,
          ],
          true,
        )
        .into(),
      );
    } else {
      // We have differing values. Set each motor separately.
      let mut i = 1;

      for cmd in cmds {
        if let Some((_, speed)) = cmd {
          msg_vec
            .push(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF3, i, *speed as u8], true).into());
        }
        i += 1;
      }
    }
    Ok(msg_vec)
  }
}
