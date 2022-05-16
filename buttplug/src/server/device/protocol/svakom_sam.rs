// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, Endpoint},
  server::{
    ButtplugServerResultFuture,
    device::{
      protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
      configuration::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder},
      hardware::{Hardware, HardwareWriteCmd},
    },
  }
};
use std::sync::Arc;

super::default_protocol_declaration!(SvakomSam, "svakom-sam");

impl ButtplugProtocolCommandHandler for SvakomSam {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::VibrateCmd,
  ) -> ButtplugServerResultFuture {
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, false)?;
      if let Some(cmds) = result {
        if let Some(speed) = cmds[0] {
          device
            .write_value(HardwareWriteCmd::new(
              Endpoint::Tx,
              [18, 1, 3, 0, 5, speed as u8].to_vec(),
              true,
            ))
            .await?;
        }
        if cmds.len() > 1 {
          if let Some(speed) = cmds[1] {
            device
              .write_value(HardwareWriteCmd::new(
                Endpoint::Tx,
                [18, 6, 1, speed as u8].to_vec(),
                true,
              ))
              .await?;
          }
        }
      }

      Ok(messages::Ok::default().into())
    })
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use crate::{
    core::messages::{Endpoint, StopDeviceCmd, VibrateCmd, VibrateSubcommand},
    server::device::{
      hardware::{HardwareCommand, HardwareWriteCmd},
      hardware::communication::test::{
        check_test_recv_empty,
        check_test_recv_value,
        new_bluetoothle_test_device,
      },
    },
    util::async_manager,
  };

  #[test]
  pub fn test_svakom_sam_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Sam Neo")
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
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![18, 1, 3, 0, 5, 5],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));
      // Since we only created one changed subcommand, we should only receive one command.
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.5),
              VibrateSubcommand::new(1, 1.0),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![18, 6, 1, 1], true)),
      );
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver));
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![18, 1, 3, 0, 5, 0],
          true,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![18, 6, 1, 0], true)),
      );
      assert!(check_test_recv_empty(&command_receiver));
    });
  }
}
