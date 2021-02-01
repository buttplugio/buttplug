use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{
    self,
    ButtplugDeviceCommandMessageUnion,
    ButtplugDeviceMessage,
    DeviceMessageAttributesMap,
    VibrateCmd,
    VibrateSubcommand,
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(ButtplugProtocolProperties)]
pub struct Vibratissimo {
  name: String,
  message_attributes: DeviceMessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl ButtplugProtocol for Vibratissimo {
  fn new_protocol(
    name: &str,
    message_attributes: DeviceMessageAttributesMap,
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

impl ButtplugProtocolCommandHandler for Vibratissimo {
  fn handle_stop_device_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::StopDeviceCmd,
  ) -> ButtplugDeviceResultFuture {
    self.handle_vibrate_cmd(
      device,
      VibrateCmd::new(
        message.device_index(),
        vec![VibrateSubcommand::new(0, 0f64)],
      ),
    )
  }

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
        // We have something to write, so push our mode command.
        fut_vec.push(device.write_value(DeviceWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )));
        for cmd in cmds.iter() {
          if let Some(speed) = cmd {
            fut_vec.push(device.write_value(DeviceWriteCmd::new(
              Endpoint::TxVibrate,
              vec![*speed as u8, 0x00],
              false,
            )));
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
    test::{check_test_recv_empty, check_test_recv_value, new_bluetoothle_test_device},
    util::async_manager,
  };

  #[test]
  pub fn test_vibratissimo_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Vibratissimo").await.unwrap();
      let command_receiver_vibrate = test_device
        .get_endpoint_receiver(&Endpoint::TxVibrate)
        .unwrap();
      let command_receiver_mode = test_device
        .get_endpoint_receiver(&Endpoint::TxMode)
        .unwrap();
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
      check_test_recv_value(
        &command_receiver_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      check_test_recv_value(
        &command_receiver_vibrate,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x80, 0x00],
          false,
        )),
      );
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
      assert!(check_test_recv_empty(&command_receiver_mode));
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .unwrap();
      check_test_recv_value(
        &command_receiver_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      check_test_recv_value(
        &command_receiver_vibrate,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x0, 0x0],
          false,
        )),
      );
    });
  }
}
