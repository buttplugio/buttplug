// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, Endpoint},
  server::device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    configuration::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder},
    hardware::{ButtplugDeviceResultFuture, Hardware, HardwareWriteCmd},
  },
};
use std::sync::Arc;

super::default_protocol_declaration!(WeVibe8Bit, "wevibe-8bit");

impl ButtplugProtocolCommandHandler for WeVibe8Bit {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // Store off result before the match, so we drop the lock ASAP.
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, true)?;
      if let Some(cmds) = result {
        let r_speed_int = cmds[0].unwrap_or(0) as u8;
        let r_speed_ext = cmds.last().unwrap_or(&None).unwrap_or(0u32) as u8;
        let data = if r_speed_int == 0 && r_speed_ext == 0 {
          vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        } else {
          let status_byte: u8 =
            (if r_speed_ext == 0 { 0 } else { 2 }) | (if r_speed_int == 0 { 0 } else { 1 });
          vec![
            0x0f,
            0x03,
            0x00,
            r_speed_ext + 3,
            r_speed_int + 3,
            status_byte,
            0x00,
            0x00,
          ]
        };
        device
          .write_value(HardwareWriteCmd::new(Endpoint::Tx, data, true))
          .await?;
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
      communication::test::{
        check_test_recv_empty,
        check_test_recv_value,
        new_bluetoothle_test_device,
      },
    },
    util::async_manager,
  };

  #[test]
  pub fn test_wevibe8bit_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Moxie")
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
          vec![0x0f, 0x03, 0x00, 0x09, 0x09, 0x03, 0x00, 0x00],
          true,
        )),
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
          vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
          true,
        )),
      );
    });
  }
}
