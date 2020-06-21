use super::{ButtplugProtocol, ButtplugProtocolCommandHandler, ButtplugProtocolCreator};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, MessageAttributesMap},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl, DeviceWriteCmd, Endpoint,
  },
  server::ButtplugServerResultFuture,
};
use async_mutex::Mutex;
use std::sync::Arc;

#[derive(ButtplugProtocol, ButtplugProtocolCreator, ButtplugProtocolProperties)]
pub struct Aneros {
  name: String,
  message_attributes: MessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl Aneros {
  pub(super) fn new(name: &str, message_attributes: MessageAttributesMap) -> Self {
    let manager = GenericCommandManager::new(&message_attributes);

    Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
    }
  }
}

impl ButtplugProtocolCommandHandler for Aneros {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
    message: messages::VibrateCmd,
  ) -> ButtplugServerResultFuture {
    // Store off result before the match, so we drop the lock ASAP.
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, false);
      let mut fut_vec = vec![];
      match result {
        Ok(cmds_option) => {
          if let Some(cmds) = cmds_option {
            let mut index = 0u8;
            for cmd in cmds {
              if let Some(speed) = cmd {
                fut_vec.push(device.write_value(DeviceWriteCmd::new(
                  Endpoint::Tx,
                  vec![0xF1 + index, speed as u8],
                  false,
                )));
              }
              index += 1;
            }
          }
          // TODO Just use join_all here
          for fut in fut_vec {
            // TODO Do something about possible errors here
            fut.await?;
          }
          Ok(messages::Ok::default().into())
        }
        Err(e) => Err(e.into()),
      }
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
  pub fn test_aneros_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Massage Demo")
        .await
        .unwrap();
      let command_receiver = test_device.get_endpoint_channel(&Endpoint::Tx).unwrap().receiver;
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 63], false)),
      )
      .await;
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
      assert!(command_receiver.is_empty());
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
      // TODO There's probably a more concise way to do this.
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 12], false)),
      )
      .await;
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF2, 63], false)),
      )
      .await;
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 0], false)),
      )
      .await;
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF2, 0], false)),
      )
      .await;
    });
  }
}
