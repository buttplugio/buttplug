use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, DeviceMessageAttributesMap},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl, DeviceWriteCmd, Endpoint,
  },
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(ButtplugProtocolProperties)]
pub struct HtkBm {
  name: String,
  message_attributes: DeviceMessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl ButtplugProtocol for HtkBm {
  fn new_protocol(
    name: &str,
    message_attributes: DeviceMessageAttributesMap,
  ) -> Box<dyn ButtplugProtocol>
  where
    Self: Sized,
  {
    let manager = GenericCommandManager::new(&message_attributes);

    Box::new(Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
    })
  }
}

impl ButtplugProtocolCommandHandler for HtkBm {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // Store off result before the match, so we drop the lock ASAP.
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, true)?;
      if let Some(cmds) = result {
        if cmds.len() >= 2 {
          let mut data: u8 = 15;
          let left = cmds[0].unwrap_or(0);
          let right = cmds[1].unwrap_or(0);
          if left != 0 && right != 0 {
            data = 11 // both (normal mode)
          } else if left != 0 {
            data = 12 // left only
          } else if right != 0 {
            data = 13 // right only
          }
          device
              .write_value(DeviceWriteCmd::new(
                Endpoint::Tx,
                vec![data],
                false,
              ))
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
    core::messages::{StopDeviceCmd, VibrateCmd, VibrateSubcommand},
    device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
    server::comm_managers::test::{check_test_recv_empty, check_test_recv_value, new_bluetoothle_test_device},
    util::async_manager,
  };

  #[test]
  pub fn test_htk_bm_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("HTK-BLE-BM001").await.unwrap();
      let command_receiver = test_device.get_endpoint_receiver(&Endpoint::Tx).unwrap();
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![12], false)),
      );
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
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
        .unwrap();
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![11], false)),
      );
      assert!(check_test_recv_empty(&command_receiver));
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .unwrap();
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![15], false)),
      );
      assert!(check_test_recv_empty(&command_receiver));
    });
  }
}
