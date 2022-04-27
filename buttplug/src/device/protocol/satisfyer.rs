use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, DeviceMessageAttributesMap},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceReadCmd,
    DeviceWriteCmd,
    Endpoint,
  },
  util::async_manager,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

#[derive(ButtplugProtocolProperties)]
pub struct Satisfyer {
  name: String,
  message_attributes: DeviceMessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  last_command: Arc<Mutex<Vec<u8>>>,
}

// Satisfyer toys will drop their connections if they don't get an update within ~10 seconds.
// Therefore we try to send a command every ~3s unless something is sent/updated sooner.
async fn send_satisfyer_updates(device: Arc<DeviceImpl>, data: Arc<Mutex<Vec<u8>>>) {
  while device.connected() {
    // Scope to make sure we drop the lock before sleeping.
    {
      let current_data = data.lock().await.clone();
      if let Err(e) = device
        .write_value(DeviceWriteCmd::new(
          Endpoint::Tx,
          current_data.clone().to_vec(),
          false,
        ))
        .await
      {
        error!(
          "Got an error from a satisfyer device, exiting control loop: {:?}",
          e
        );
        break;
      }
    }
    tokio::time::sleep(Duration::from_secs(1)).await;
  }
}

impl Satisfyer {
  fn new(
    name: &str,
    message_attributes: DeviceMessageAttributesMap,
    last_command: Arc<Mutex<Vec<u8>>>,
  ) -> Self {
    let manager = GenericCommandManager::new(&message_attributes);
    Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
      last_command,
    }
  }
}

impl ButtplugProtocol for Satisfyer {
  fn try_create(
    device_impl: Arc<crate::device::DeviceImpl>,
    config: crate::device::protocol::DeviceProtocolConfiguration,
  ) -> futures::future::BoxFuture<
    'static,
    Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>,
  > {
    let msg = DeviceWriteCmd::new(Endpoint::Command, vec![0x01], true);
    let info_fut = device_impl.write_value(msg);

    Box::pin(async move {
      let result = device_impl
        .read_value(DeviceReadCmd::new(Endpoint::RxBLEModel, 128, 500))
        .await?;
      let device_identifier = format!(
        "{}",
        u32::from_be_bytes(result.data().to_vec().try_into().unwrap_or([0; 4]))
      );
      info!(
        "Satisfyer Device Identifier: {:?} {}",
        result.data(),
        device_identifier
      );
      info_fut.await?;
      let (name, attrs) = crate::device::protocol::get_protocol_features(
        device_impl.clone(),
        Some(device_identifier),
        config,
      )?;
      // Now that we've initialized and constructed the device, start the update cycle to make sure
      // we don't drop the connection.
      let last_command = Arc::new(Mutex::new(vec![0u8; 8]));
      let device = Self::new(&name, attrs, last_command.clone());
      async_manager::spawn(async move {
        send_satisfyer_updates(device_impl, last_command).await;
      });
      Ok(Box::new(device) as Box<dyn ButtplugProtocol>)
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
    let last_command = self.last_command.clone();
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
        *last_command.lock().await = data.clone();
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

  // TODO Reimplement with name readout and timing fixes (#414)
  #[test]
  #[ignore]
  pub fn test_satisfyer_2v_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("SF Curvy 2+")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .get_endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      /*
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0, 0, 0, 0, 0, 0, 0, 0],
          false,
        )),
      );
       */
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

  // TODO Reimplement with name readout and timing fixes (#414)
  #[test]
  #[ignore]
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
