use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::{
    errors::ButtplugError,
    messages::{
      self, ButtplugDeviceCommandMessageUnion, ButtplugDeviceMessage, DeviceMessageAttributesMap,
      VibrateCmd, VibrateSubcommand,
    },
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl, DeviceReadCmd, DeviceWriteCmd, Endpoint,
  },
};
use std::sync::Arc;
use tokio::sync::Mutex;
use futures::future::BoxFuture;

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

  fn initialize(
    device_impl: Arc<DeviceImpl>,
  ) -> BoxFuture<'static, Result<Option<String>, ButtplugError>> {
    Box::pin(async move {

      let mut name = device_impl.name.clone();
      let result = device_impl.read_value(DeviceReadCmd::new(Endpoint::RxBLEModel, 128, 500)).await;
      if result.is_ok() {
        name = String::from_utf8(result.unwrap().data().to_vec()).unwrap_or(device_impl.name.clone());
      }

      Ok(Some(name))
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
      let result = manager.lock().await.update_vibration(&message, true)?;
      let mut fut_vec = vec![];
      if let Some(cmds) = result {
        let mut data: Vec<u8> = Vec::new();
        for i in 0..cmds.len() {
          data.push( cmds[i].unwrap_or(0) as u8 );
        }
        if data.len() == 1 {
          data.push( 0x00 );
        }

        // Put the device in write mode
        fut_vec.push(device.write_value(DeviceWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )));
        fut_vec.push(device.write_value(DeviceWriteCmd::new(
          Endpoint::TxVibrate,
          data,
          false,
        )));
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
    server::comm_managers::test::{check_test_recv_empty, check_test_recv_value, new_bluetoothle_test_device},
    util::async_manager,
  };

  #[test]
  pub fn test_vibratissimo_protocol_default() {
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
        &command_receiver_vibrate,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x80, 0x00],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));

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
        &command_receiver_vibrate,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x0, 0x0],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));
    });
  }

  #[test]
  #[ignore] // Need to be able to set BLE model info to be read on test device
  pub fn test_vibratissimo_protocol_licker() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Vibratissimo").await.unwrap();
      let command_receiver_vibrate = test_device
          .get_endpoint_receiver(&Endpoint::TxVibrate)
          .unwrap();
      let command_receiver_mode = test_device
          .get_endpoint_receiver(&Endpoint::TxMode)
          .unwrap();

      assert!(check_test_recv_empty(&command_receiver_mode));
      assert!(check_test_recv_empty(&command_receiver_vibrate));

      device
          .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
          .await
          .unwrap();
      check_test_recv_value(
        &command_receiver_vibrate,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x80, 0x00],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));

      device
          .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(1, 1.0)]).into())
          .await
          .unwrap();
      check_test_recv_value(
        &command_receiver_vibrate,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x80, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));

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
        &command_receiver_vibrate,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x0, 0x0],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));
    });
  }

  #[test]
  #[ignore] // Need to be able to set BLE model info to be read on test device
  pub fn test_vibratissimo_protocol_rabbit() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Vibratissimo").await.unwrap();
      let command_receiver_vibrate = test_device
          .get_endpoint_receiver(&Endpoint::TxVibrate)
          .unwrap();
      let command_receiver_mode = test_device
          .get_endpoint_receiver(&Endpoint::TxMode)
          .unwrap();

      assert!(check_test_recv_empty(&command_receiver_mode));
      assert!(check_test_recv_empty(&command_receiver_vibrate));

      device
          .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
          .await
          .unwrap();
      check_test_recv_value(
        &command_receiver_vibrate,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x80, 0x00, 0x0],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));

      device
          .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(1, 1.0)]).into())
          .await
          .unwrap();
      check_test_recv_value(
        &command_receiver_vibrate,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x80, 0xff, 0x0],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));

      device
          .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(2, 1.0)]).into())
          .await
          .unwrap();
      check_test_recv_value(
        &command_receiver_vibrate,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x80, 0xff, 0x02],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));

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
        &command_receiver_vibrate,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x0, 0x0, 0x0],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));
    });
  }
}
