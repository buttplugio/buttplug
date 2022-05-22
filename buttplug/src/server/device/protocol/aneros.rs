// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    messages::{Endpoint},
  },
  server::{
    device::{
      hardware::{HardwareWriteCmd, HardwareCommand},
      protocol::{
        GenericProtocolIdentifier, ProtocolIdentifier, ProtocolIdentifierFactory, ProtocolHandler
      },
    },
  },
};


#[derive(Default)]
pub struct AnerosIdentifierFactory {}

impl ProtocolIdentifierFactory for AnerosIdentifierFactory {
  fn identifier(&self) -> &str {
    "aneros"
  }

  fn create(&self) -> Box<dyn ProtocolIdentifier> {
    Box::new(GenericProtocolIdentifier::new(Box::new(Aneros::default()), self.identifier()))
  }
}

#[derive(Default)]
pub struct Aneros {}

impl ProtocolHandler for Aneros {
  fn handle_vibrate_cmd(&self, cmds: &Vec<Option<u32>>) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut cmd_vec = vec!();
    for (index, cmd) in cmds.iter().enumerate() {
      if let Some(speed) = cmd {
        cmd_vec.push(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0xF1 + (index as u8), *speed as u8],
          false,
        ).into());
      }
    }
    Ok(cmd_vec)
  }
}
/*
#[cfg(all(test, feature = "server"))]
mod test {
  use crate::{
    core::messages::{Endpoint, StopDeviceCmd, VibrateCmd, VibrateSubcommand},
    server::device::{
      hardware::communication::test::{
        check_test_recv_empty, check_test_recv_value, new_bluetoothle_test_device,
      },
      hardware::{HardwareCommand, HardwareWriteCmd},
    },
    util::async_manager,
  };

  #[test]
  pub fn test_aneros_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Massage Demo")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF1, 64], false)),
      );
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver));
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.1),
              VibrateSubcommand::new(1, 0.5),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      // TODO There's probably a more concise way to do this.
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF1, 13], false)),
      );
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF2, 64], false)),
      );
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF1, 0], false)),
      );
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF2, 0], false)),
      );
    });
  }
}
 */