use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    configuration_manager::{ProtocolDeviceAttributes, DeviceAttributesBuilder},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use futures_timer::Delay;
use std::sync::Arc;
use std::time::Duration;

super::default_protocol_definition!(WeVibe, "wevibe");

#[derive(Default, Debug)]
pub struct WeVibeFactory {}

impl ButtplugProtocolFactory for WeVibeFactory {
  fn try_create(
    &self,
    device_impl: Arc<crate::device::DeviceImpl>,
    builder: DeviceAttributesBuilder,
  ) -> futures::future::BoxFuture<
    'static,
    Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>,
  > {
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
      let device_attributes = builder.create_from_impl(&device_impl)?;
      Ok(Box::new(WeVibe::new(device_attributes)) as Box<dyn ButtplugProtocol>)
    })
  }

  fn protocol_identifier(&self) -> &'static str {
    "wevibe"
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
    server::comm_managers::test::{
      check_test_recv_empty,
      check_test_recv_value,
      new_bluetoothle_test_device,
    },
    util::async_manager,
  };

  #[test]
  pub fn test_wevibe_protocol_two_features() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("4 Plus")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
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
      let (device, test_device) = new_bluetoothle_test_device("Ditto")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
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
