use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, MessageAttributesMap},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use async_lock::Mutex;
use std::sync::Arc;

#[derive(ButtplugProtocolProperties)]
pub struct LiboVibes {
  name: String,
  message_attributes: MessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl ButtplugProtocol for LiboVibes {
  fn new_protocol(
    name: &str,
    message_attributes: MessageAttributesMap,
  ) -> Box<dyn ButtplugProtocol> {
    let manager = GenericCommandManager::new(&message_attributes);

    Box::new(Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
    })
  }
}

impl ButtplugProtocolCommandHandler for LiboVibes {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
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

#[cfg(test)]
mod test {
  use crate::{
    core::messages::{StopDeviceCmd, VibrateCmd, VibrateSubcommand},
    device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
    test::{check_recv_value, new_bluetoothle_test_device},
    util::async_manager,
  };

  #[test]
  pub fn test_libo_vibes_protocol_1vibe() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Yuyi").await.unwrap();
      let command_receiver_tx = test_device
        .get_endpoint_channel(&Endpoint::Tx)
        .unwrap()
        .receiver;
      let command_receiver_tx_mode = test_device
        .get_endpoint_channel(&Endpoint::TxMode)
        .unwrap()
        .receiver;
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver_tx,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0x32], false)),
      )
      .await;
      assert!(command_receiver_tx.is_empty());
      assert!(command_receiver_tx_mode.is_empty());

      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
      assert!(command_receiver_tx.is_empty());
      assert!(command_receiver_tx_mode.is_empty());

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver_tx,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0x00], false)),
      )
      .await;
      assert!(command_receiver_tx.is_empty());
      check_recv_value(
        &command_receiver_tx_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::TxMode, vec![0x00], false)),
      )
      .await;
      assert!(command_receiver_tx_mode.is_empty());
    });
  }
  #[test]
  pub fn test_libo_vibes_protocol_2vibe() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Gugudai").await.unwrap();
      let command_receiver_tx = test_device
        .get_endpoint_channel(&Endpoint::Tx)
        .unwrap()
        .receiver;
      let command_receiver_tx_mode = test_device
        .get_endpoint_channel(&Endpoint::TxMode)
        .unwrap()
        .receiver;
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
        .unwrap();
      check_recv_value(
        &command_receiver_tx,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0x32], false)),
      )
      .await;
      assert!(command_receiver_tx.is_empty());
      check_recv_value(
        &command_receiver_tx_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::TxMode, vec![0x02], false)),
      )
      .await;
      assert!(command_receiver_tx_mode.is_empty());

      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(1, 1.0)]).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver_tx_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::TxMode, vec![0x03], false)),
      )
      .await;
      assert!(command_receiver_tx_mode.is_empty());
      assert!(command_receiver_tx.is_empty());

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
      assert!(command_receiver_tx.is_empty());
      assert!(command_receiver_tx_mode.is_empty());

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver_tx,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0x00], false)),
      )
      .await;
      assert!(command_receiver_tx.is_empty());
      check_recv_value(
        &command_receiver_tx_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::TxMode, vec![0x00], false)),
      )
      .await;
      assert!(command_receiver_tx_mode.is_empty());
    });
  }
}
