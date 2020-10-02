use super::{
  ButtplugDeviceResultFuture,
  ButtplugProtocol,
  ButtplugProtocolCommandHandler,
  ButtplugProtocolCreator,
};
use crate::{
  core::{
    errors::ButtplugError,
    messages::{
      self,
      ButtplugDeviceCommandMessageUnion,
      MessageAttributesMap
    },
  },
  device::{
    configuration_manager::DeviceProtocolConfiguration,
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use async_mutex::Mutex;
use futures::future::BoxFuture;
use futures_timer::Delay;
use std::sync::Arc;
use std::time::Duration;

#[derive(ButtplugProtocol, ButtplugProtocolProperties)]
pub struct WeVibe {
  name: String,
  message_attributes: MessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl WeVibe {
  pub(super) fn new(name: &str, message_attributes: MessageAttributesMap) -> Self {
    let manager = GenericCommandManager::new(&message_attributes);

    Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
    }
  }
}

impl ButtplugProtocolCreator for WeVibe {
  fn new_protocol(name: &str, attrs: MessageAttributesMap) -> Box<dyn ButtplugProtocol> {
    Box::new(Self::new(name, attrs))
  }

  fn try_create(
    device_impl: &dyn DeviceImpl,
    configuration: DeviceProtocolConfiguration,
  ) -> BoxFuture<'static, Result<Box<dyn ButtplugProtocol>, ButtplugError>> {
    debug!("calling WeVibe init");
    let vibration_on = device_impl.write_value(DeviceWriteCmd::new(
      Endpoint::Tx,
      vec![0x0f, 0x03, 0x00, 0x99, 0x00, 0x03, 0x00, 0x00],
      false
    ));
    let vibration_off = device_impl.write_value(DeviceWriteCmd::new(
      Endpoint::Tx,
      vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
      false
    ));
    let device_name = device_impl.name().to_owned();
    Box::pin(async move {
      vibration_on.await?;
      Delay::new(Duration::from_millis(100)).await;
      vibration_off.await?;
      let (names, attrs) = configuration.get_attributes(&device_name).unwrap();
      let name = names.get("en-us").unwrap();
      Ok(Self::new_protocol(name, attrs))
    })
  }
}

impl ButtplugProtocolCommandHandler for WeVibe {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
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
          .write_value(DeviceWriteCmd::new(Endpoint::Tx, data, false))
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
  pub fn test_wevibe_protocol_two_features() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("4 Plus").await.unwrap();
      let command_receiver = test_device
        .get_endpoint_channel(&Endpoint::Tx)
        .unwrap()
        .receiver;
      check_recv_value(
          &command_receiver,
          DeviceImplCommand::Write(DeviceWriteCmd::new(
            Endpoint::Tx,
            vec![0x0f, 0x03, 0x00, 0x99, 0x00, 0x03, 0x00, 0x00],
            false,
          )),
        )
        .await;
      check_recv_value(
          &command_receiver,
          DeviceImplCommand::Write(DeviceWriteCmd::new(
            Endpoint::Tx,
            vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            false,
          )),
        )
        .await;  
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x0f, 0x03, 0x00, 0x80, 0x00, 0x03, 0x00, 0x00],
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
              VibrateSubcommand::new(0, 0.25),
              VibrateSubcommand::new(1, 0.75),
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
          vec![0x0f, 0x03, 0x00, 0x4c, 0x00, 0x03, 0x00, 0x00],
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
          vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
          false,
        )),
      )
      .await;
    });
  }

  #[test]
  pub fn test_wevibe_protocol_one_feature() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Ditto").await.unwrap();
      let command_receiver = test_device
        .get_endpoint_channel(&Endpoint::Tx)
        .unwrap()
        .receiver;
      check_recv_value(
          &command_receiver,
          DeviceImplCommand::Write(DeviceWriteCmd::new(
            Endpoint::Tx,
            vec![0x0f, 0x03, 0x00, 0x99, 0x00, 0x03, 0x00, 0x00],
            false,
          )),
        )
        .await;
      check_recv_value(
          &command_receiver,
          DeviceImplCommand::Write(DeviceWriteCmd::new(
            Endpoint::Tx,
            vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            false,
          )),
        )
        .await;        
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x0f, 0x03, 0x00, 0x88, 0x00, 0x03, 0x00, 0x00],
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
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
          false,
        )),
      )
      .await;
    });
  }
}
