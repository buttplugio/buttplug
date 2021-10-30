use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::{
    errors::ButtplugError,
    messages::{self, ButtplugDeviceCommandMessageUnion, DeviceMessageAttributesMap},
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use futures::future::BoxFuture;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(ButtplugProtocolProperties)]
pub struct LoveDistance {
  name: String,
  message_attributes: DeviceMessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl ButtplugProtocol for LoveDistance {
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

  fn initialize(
    device_impl: Arc<DeviceImpl>,
  ) -> BoxFuture<'static, Result<Option<String>, ButtplugError>> {
    Box::pin(async move {
      let msg = DeviceWriteCmd::new(Endpoint::Tx, vec![0xf3, 0, 0], false);
      device_impl.write_value(msg).await?;
      let msg = DeviceWriteCmd::new(Endpoint::Tx, vec![0xf4, 1], false);
      device_impl.write_value(msg).await?;
      Ok(None)
    })
  }
}

impl ButtplugProtocolCommandHandler for LoveDistance {
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
          device
            .write_value(DeviceWriteCmd::new(
              Endpoint::Tx,
              vec![0xf3, 0x00, speed as u8],
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
    server::comm_managers::test::{check_test_recv_value, new_bluetoothle_test_device},
    util::async_manager,
  };

  #[test]
  pub fn test_lovedistance_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("REACH G")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .get_endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xf3, 0, 0], false)),
      );
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xf4, 01], false)),
      );

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0xf3, 0, 0x3d],
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
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xf3, 0, 0], false)),
      );
    });
  }
}
