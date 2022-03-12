use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
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

super::default_protocol_declaration!(LiboVibes);

impl ButtplugProtocolCommandHandler for LiboVibes {
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
            if index == 0 {
              fut_vec.push(device.write_value(DeviceWriteCmd::new(
                Endpoint::Tx,
                vec![*speed as u8],
                false,
              )));

              // If this is a single vibe device, we need to send stop to TxMode too
              if *speed as u8 == 0 && cmds.len() == 1 {
                fut_vec.push(device.write_value(DeviceWriteCmd::new(
                  Endpoint::TxMode,
                  vec![0u8],
                  false,
                )));
              }
            } else if index == 1 {
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
  pub fn test_libo_vibes_protocol_1vibe() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Yuyi")
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
        &command_receiver_tx,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0x32], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_tx_mode));

      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_tx_mode));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0x00], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      check_test_recv_value(
        &command_receiver_tx_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::TxMode, vec![0x00], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx_mode));
    });
  }
  #[test]
  pub fn test_libo_vibes_protocol_2vibe() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Gugudai")
        .await
        .expect("Test, assuming infallible");
      let command_receiver_tx = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      let command_receiver_tx_mode = test_device
        .endpoint_receiver(&Endpoint::TxMode)
        .expect("Test, assuming infallible");
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.5),
              VibrateSubcommand::new(1, 0.5),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0x32], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      check_test_recv_value(
        &command_receiver_tx_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::TxMode, vec![0x02], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx_mode));

      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(1, 1.0)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::TxMode, vec![0x03], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx_mode));
      assert!(check_test_recv_empty(&command_receiver_tx));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_tx_mode));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0x00], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      check_test_recv_value(
        &command_receiver_tx_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::TxMode, vec![0x00], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx_mode));
    });
  }
}
