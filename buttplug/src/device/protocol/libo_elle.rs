// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    configuration_manager::{ProtocolDeviceAttributes, DeviceAttributesBuilder},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use std::sync::Arc;

super::default_protocol_declaration!(LiboElle, "libo-elle");

impl ButtplugProtocolCommandHandler for LiboElle {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // Store off result before the match, so we drop the lock ASAP.
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, false)?;
      let mut fut_vec = vec![];
      if let Some(cmds) = result {
        for (index, cmd) in cmds.iter().enumerate() {
          if let Some(speed) = cmd {
            if index == 1 {
              let mut data = 0u8;
              if *speed as u8 > 0 && *speed as u8 <= 7 {
                data |= (*speed as u8 - 1) << 4;
                data |= 1; // Set the mode too
              } else if *speed as u8 > 7 {
                data |= (*speed as u8 - 8) << 4;
                data |= 4; // Set the mode too
              }
              fut_vec.push(device.write_value(DeviceWriteCmd::new(
                Endpoint::Tx,
                vec![data],
                false,
              )));
            } else if index == 0 {
              fut_vec.push(device.write_value(DeviceWriteCmd::new(
                Endpoint::TxMode,
                vec![*speed as u8],
                false,
              )));
            }
          }
        }
      }
      // TODO Just use join_all here
      for fut in fut_vec {
        // TODO Do something about possible errors here
        fut.await?;
      }
      Ok(messages::Ok::default().into())
    })
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use crate::{
    core::messages::{StopDeviceCmd, VibrateCmd, VibrateSubcommand},
    device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
    server::comm_managers::test::{
      check_test_recv_empty,
      check_test_recv_value,
      new_bluetoothle_test_device,
    },
    util::async_manager,
  };

  #[test]
  pub fn test_libo_elle_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("PiPiJing")
        .await
        .expect("Test, assuming infallible");
      let command_receiver_tx = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      let command_receiver_tx_mode = test_device
        .endpoint_receiver(&Endpoint::TxMode)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::TxMode, vec![0x02], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx_mode));
      assert!(check_test_recv_empty(&command_receiver_tx));

      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 1.0)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::TxMode, vec![0x03], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx_mode));
      assert!(check_test_recv_empty(&command_receiver_tx));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 1.0)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_tx_mode));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::TxMode, vec![0x00], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx_mode));
      assert!(check_test_recv_empty(&command_receiver_tx));
    });
  }
}
