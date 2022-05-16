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
    hardware::device_impl::{ButtplugDeviceResultFuture, DeviceImpl, DeviceWriteCmd},
  },
};
use std::sync::Arc;

super::default_protocol_declaration!(MagicMotionV3, "magic-motion-3");

impl ButtplugProtocolCommandHandler for MagicMotionV3 {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // Store off result before the match, so we drop the lock ASAP.
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, false)?;
      if let Some(cmds) = result {
        device
          .write_value(DeviceWriteCmd::new(
            Endpoint::Tx,
            vec![
              0x0b,
              0xff,
              0x04,
              0x0a,
              0x46,
              0x46,
              0x00,
              0x04,
              0x08,
              cmds[0].unwrap_or(0) as u8,
              0x64,
              0x00,
            ],
            false,
          ))
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
      hardware::device_impl::{DeviceImplCommand, DeviceWriteCmd},
      communication::test::{
        check_test_recv_empty,
        check_test_recv_value,
        new_bluetoothle_test_device,
      },
    },
    util::async_manager,
  };

  #[test]
  pub fn test_magic_motion_v3_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Krush")
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
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![
            0x0b, 0xff, 0x04, 0x0a, 0x46, 0x46, 0x00, 0x04, 0x08, 0x27, 0x64, 0x00,
          ],
          false,
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
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![
            0x0b, 0xff, 0x04, 0x0a, 0x46, 0x46, 0x00, 0x04, 0x08, 0x00, 0x64, 0x00,
          ],
          false,
        )),
      );
    });
  }
}
