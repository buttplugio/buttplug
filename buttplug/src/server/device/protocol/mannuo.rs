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

super::default_protocol_declaration!(ManNuo, "mannuo");

impl ButtplugProtocolCommandHandler for ManNuo {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::VibrateCmd,
  ) -> ButtplugServerResultFuture {
    // Store off result before the match, so we drop the lock ASAP.
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, false)?;
      if let Some(cmds) = result {
        if !cmds.is_empty() {
          if let Some(speed) = cmds[0] {
            let mut data = vec![0xAA, 0x55, 0x06, 0x01, 0x01, 0x01, speed as u8, 0xFA];

            // Simple XOR of everything up to the 9th byte for CRC.
            let mut crc: u8 = 0;
            for b in data.clone() {
              crc ^= b;
            }
            data.push(crc);

            device
              .write_value(HardwareWriteCmd::new(Endpoint::Tx, data, true))
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
    core::messages::{Endpoint, VibrateCmd, VibrateSubcommand},
    server::device::{
      hardware::{HardwareCommand, HardwareWriteCmd},
    hardware::communication::test::{
      check_test_recv_empty,
      check_test_recv_value,
      new_bluetoothle_test_device,
    }},
    util::async_manager,
  };

  #[test]
  pub fn test_mannuo_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Sex toys")
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
          vec![170, 85, 6, 1, 1, 1, 2, 250, 0],
          true,
        )),
      );
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver));
    });
  }
}
