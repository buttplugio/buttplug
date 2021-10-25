use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::{
    errors::ButtplugError,
    messages::{self, ButtplugDeviceCommandMessageUnion, DeviceMessageAttributesMap},
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl, DeviceWriteCmd, Endpoint,
  },
};
use futures::future::BoxFuture;
use futures_timer::Delay;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(ButtplugProtocolProperties)]
pub struct WeVibe {
  name: String,
  message_attributes: DeviceMessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl ButtplugProtocol for WeVibe {
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
    debug!("calling WeVibe init");
    let vibration_on = device_impl.write_value(DeviceWriteCmd::new(
      Endpoint::Tx,
      vec![0x0f, 0x03, 0x00, 0x99, 0x00, 0x03, 0x00, 0x00],
      true,
    ));
    let vibration_off = device_impl.write_value(DeviceWriteCmd::new(
      Endpoint::Tx,
      vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
      true,
    ));
    Box::pin(async move {
      vibration_on.await?;
      Delay::new(Duration::from_millis(100)).await;
      vibration_off.await?;
      Ok(None)
    })
  }
}

impl ButtplugProtocolCommandHandler for WeVibe {
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
        let r_speed_int = cmds[0].unwrap_or(0) as u8;
        let r_speed_ext = cmds.last().unwrap_or(&None).unwrap_or(0u32) as u8;
        let data = if r_speed_int == 0 && r_speed_ext == 0 {
          vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        } else {
          vec![
            0x0f,
            0x03,
            0x00,
            r_speed_ext | (r_speed_int << 4),
            0x00,
            0x03,
            0x00,
            0x00,
          ]
        };
        device
          .write_value(DeviceWriteCmd::new(Endpoint::Tx, data, true))
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
    server::comm_managers::test::{check_test_recv_empty, check_test_recv_value, new_bluetoothle_test_device},
    util::async_manager,
  };

  #[test]
  pub fn test_wevibe_protocol_two_features() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("4 Plus").await.expect("Test, assuming infallible");
      let command_receiver = test_device.get_endpoint_receiver(&Endpoint::Tx).expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x0f, 0x03, 0x00, 0x99, 0x00, 0x03, 0x00, 0x00],
          true,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
          true,
        )),
      );
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x0f, 0x03, 0x00, 0x80, 0x00, 0x03, 0x00, 0x00],
          true,
        )),
      );
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver));
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.25),
              VibrateSubcommand::new(1, 0.75),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      // TODO There's probably a more concise way to do this.
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x0f, 0x03, 0x00, 0x4c, 0x00, 0x03, 0x00, 0x00],
          true,
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
          vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
          true,
        )),
      );
    });
  }

  #[test]
  pub fn test_wevibe_protocol_one_feature() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Ditto").await.expect("Test, assuming infallible");
      let command_receiver = test_device.get_endpoint_receiver(&Endpoint::Tx).expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x0f, 0x03, 0x00, 0x99, 0x00, 0x03, 0x00, 0x00],
          true,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
          true,
        )),
      );
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x0f, 0x03, 0x00, 0x88, 0x00, 0x03, 0x00, 0x00],
          true,
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
          vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
          true,
        )),
      );
    });
  }
}
