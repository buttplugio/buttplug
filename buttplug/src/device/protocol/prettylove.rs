use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::core::errors::ButtplugError;
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, MessageAttributesMap},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use futures::future::{self, BoxFuture};
use std::sync::Arc;
use async_lock::Mutex;

#[derive(ButtplugProtocolProperties)]
pub struct PrettyLove {
  name: String,
  manager: Arc<Mutex<GenericCommandManager>>,
  message_attributes: MessageAttributesMap,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl ButtplugProtocol for PrettyLove {
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

  fn initialize(
    _device_impl: &dyn DeviceImpl,
  ) -> BoxFuture<'static, Result<Option<String>, ButtplugError>> {
    // Pretty Love devices have wildcarded names of Aogu BLE *
    // Force the identifier lookup to "Aogu BLE"
    Box::pin(future::ready(Ok(Some("Aogu BLE".to_owned()))))
  }
}

impl ButtplugProtocolCommandHandler for PrettyLove {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
    message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // Store off result before the match, so we drop the lock ASAP.
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, false)?;
      if let Some(cmds) = result {
        if let Some(speed) = cmds[0] {
          device.write_value(DeviceWriteCmd::new(
            Endpoint::Tx,
            vec![0x00, speed as u8],
            false,
          )).await?;
        }
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
  pub fn test_prettylove_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Aogu BLE Device").await.unwrap();
      let command_receiver = test_device
          .get_endpoint_channel(&Endpoint::Tx)
          .unwrap()
          .receiver;
      device
          .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
          .await
          .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0x00, 0x02], false)),
      )
          .await;
      assert!(command_receiver.is_empty());

      // Since we only created one subcommand, we should only receive one command.
      device
          .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
          .await
          .unwrap();
      assert!(command_receiver.is_empty());

      device
          .parse_message(StopDeviceCmd::new(0).into())
          .await
          .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0x00, 0x00], false)),
      )
          .await;
      assert!(command_receiver.is_empty());
    });
  }
}
