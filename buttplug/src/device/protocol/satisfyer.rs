use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::{
    messages::{self, ButtplugDeviceCommandMessageUnion, DeviceMessageAttributesMap},
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceReadCmd,
    DeviceWriteCmd,
    Endpoint,
  },
};
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(ButtplugProtocolProperties)]
pub struct Satisfyer {
  name: String,
  message_attributes: DeviceMessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl Satisfyer {
  fn new(
    name: &str,
    message_attributes: DeviceMessageAttributesMap,
  ) -> Self {
    let manager = GenericCommandManager::new(&message_attributes);

    Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
    }
  }
}

impl ButtplugProtocol for Satisfyer {
  fn try_create(
    device_impl: Arc<crate::device::DeviceImpl>,
    config: crate::device::protocol::DeviceProtocolConfiguration,
  ) -> futures::future::BoxFuture<'static, Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>>
  {
    let msg = DeviceWriteCmd::new(Endpoint::Command, vec![0x01], true);
    let info_fut = device_impl.write_value(msg);
    Box::pin(async move {
      let result = device_impl
        .read_value(DeviceReadCmd::new(Endpoint::RxBLEModel, 128, 500))
        .await?;
      let device_identifier =
        String::from_utf8(result.data().to_vec()).unwrap_or_else(|_| device_impl.name.clone());
      info!("Satisfyer Device Identifier: {}", device_identifier);
      info_fut.await?;
      let (name, attrs) = crate::device::protocol::get_protocol_features(device_impl, Some(device_identifier), config)?;
      Ok(Box::new(Self::new(&name, attrs)) as Box<dyn ButtplugProtocol>)
    })
  }
}

impl ButtplugProtocolCommandHandler for Satisfyer {
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
        let data = if cmds.len() == 1 {
          vec![
            cmds[0].unwrap_or(0) as u8,
            cmds[0].unwrap_or(0) as u8,
            cmds[0].unwrap_or(0) as u8,
            cmds[0].unwrap_or(0) as u8,
            0x00,
            0x00,
            0x00,
            0x00,
          ]
        } else {
          vec![
            cmds[1].unwrap_or(0) as u8,
            cmds[1].unwrap_or(0) as u8,
            cmds[1].unwrap_or(0) as u8,
            cmds[1].unwrap_or(0) as u8,
            cmds[0].unwrap_or(0) as u8,
            cmds[0].unwrap_or(0) as u8,
            cmds[0].unwrap_or(0) as u8,
            cmds[0].unwrap_or(0) as u8,
          ]
        };
        device
          .write_value(DeviceWriteCmd::new(Endpoint::Tx, data, false))
          .await?;
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
  pub fn test_satisfyer_2v_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("SF Love Triangle")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .get_endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0, 0, 0, 0, 50, 50, 50, 50],
          false,
        )),
      );
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver));
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(1, 0.9)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![90, 90, 90, 90, 50, 50, 50, 50],
          false,
        )),
      );
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0, 0, 0, 0, 0, 0, 0, 0],
          false,
        )),
      );
    });
  }

  #[test]
  pub fn test_satisfyer_1v_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("SF Royal One")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .get_endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0, 0, 0, 0, 50, 50, 50, 50],
          false,
        )),
      );
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver));
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0, 0, 0, 0, 0, 0, 0, 0],
          false,
        )),
      );
    });
  }
}
