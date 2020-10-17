use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::core::errors::ButtplugError;
use crate::device::DeviceSubscribeCmd;
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, MessageAttributesMap},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use async_mutex::Mutex;
use futures::future::BoxFuture;
use std::sync::Arc;

#[derive(ButtplugProtocolProperties)]
pub struct LeloF1s {
  name: String,
  message_attributes: MessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl ButtplugProtocol for LeloF1s {
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
    device_impl: &dyn DeviceImpl,
  ) -> BoxFuture<'static, Result<Option<String>, ButtplugError>> {
    // The Lelo F1s needs you to hit the power button after connection
    // before it'll accept any commands. Unless we listen for event on
    // the button, this is more likely to turn the device off.
    let subscribe_fut = device_impl.subscribe(DeviceSubscribeCmd::new(Endpoint::Rx));

    Box::pin(async move {
      subscribe_fut.await?;
      Ok(None)
    })
  }
}

impl ButtplugProtocolCommandHandler for LeloF1s {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
    message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // Store off result before the match, so we drop the lock ASAP.
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, true)?;
      let mut cmd_vec = vec![0x1];
      if let Some(cmds) = result {
        info!("{:?}", cmds);
        for cmd in cmds.iter() {
          cmd_vec.push(cmd.unwrap() as u8);
        }
        device
          .write_value(DeviceWriteCmd::new(Endpoint::Tx, cmd_vec, false))
          .await?;
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
  pub fn test_lelof1s_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("F1s").await.unwrap();
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
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x32, 0x0],
          false,
        )),
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
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x1, 0xa, 0x32],
          false,
        )),
      )
      .await;
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x1, 0x0, 0x0],
          false,
        )),
      )
      .await;
    });
  }
}
