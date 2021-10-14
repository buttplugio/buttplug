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
pub struct SvakomSam {
  name: String,
  manager: Arc<Mutex<GenericCommandManager>>,
  message_attributes: DeviceMessageAttributesMap,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl ButtplugProtocol for SvakomSam {
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

impl ButtplugProtocolCommandHandler for SvakomSam {
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
          device.write_value(DeviceWriteCmd::new(
            Endpoint::Tx,
            [18, 1, 3, 0, 5, speed as u8].to_vec(),
            true,
          )).await?;
        }
        if cmds.len() > 1 {
          if let Some(speed) = cmds[1] {
            device.write_value(DeviceWriteCmd::new(
              Endpoint::Tx,
              [18, 6, 1, speed as u8].to_vec(),
              true,
            )).await?;
          }
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
  pub fn test_svakom_sam_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Sam Neo").await.unwrap();
      let command_receiver = test_device.get_endpoint_receiver(&Endpoint::Tx).unwrap();
      device
          .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
          .await
          .unwrap();
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![18, 1, 3, 0, 5, 5],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));
      // Since we only created one changed subcommand, we should only receive one command.
      device
          .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5), VibrateSubcommand::new(1, 1.0)]).into())
          .await
          .unwrap();
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![18, 6, 1, 1],
          true,
        )),
      );
      // Since we only created one subcommand, we should only receive one command.
      device
          .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
          .await
          .unwrap();
      assert!(check_test_recv_empty(&command_receiver));
      device
          .parse_message(StopDeviceCmd::new(0).into())
          .await
          .unwrap();
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![18, 1, 3, 0, 5, 0],
          true,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![18, 6, 1, 0],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));
    });
  }
}