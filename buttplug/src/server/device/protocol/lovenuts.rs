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

super::default_protocol_declaration!(LoveNuts, "lovenuts");

impl ButtplugProtocolCommandHandler for LoveNuts {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, false)?;
      if let Some(cmds) = result {
        if let Some(speed) = cmds[0] {
          let mut data: Vec<u8> = vec![0x45, 0x56, 0x4f, 0x4c];
          for _ in 0..10 {
            let mut b: u8 = speed as u8;
            b |= (speed as u8) << 4;
            data.push(b);
          }
          data.push(0x00);
          data.push(0xff);

          device
            .write_value(DeviceWriteCmd::new(Endpoint::Tx, data, false))
            .await?;
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
      communication::test::{check_test_recv_value, new_bluetoothle_test_device},
      hardware::device_impl::{DeviceImplCommand, DeviceWriteCmd},
    },
    util::async_manager,
  };

  #[test]
  pub fn test_love_nuts_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Love_Nuts")
        .await
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![
            0x45, 0x56, 0x4f, 0x4c, 0x88, 0x88, 0x88, 0x88, 0x88, 0x88, 0x88, 0x88, 0x88, 0x88,
            0x00, 0xff,
          ],
          false,
        )),
      );
      // Test to make sure we handle packet IDs across protocol clones correctly.
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![
            0x45, 0x56, 0x4f, 0x4c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0xff,
          ],
          false,
        )),
      );
    });
  }
}
