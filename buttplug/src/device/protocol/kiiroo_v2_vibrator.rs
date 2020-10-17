use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
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
use std::sync::Arc;

#[derive(ButtplugProtocolProperties)]
pub struct KiirooV2Vibrator {
  name: String,
  message_attributes: MessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl ButtplugProtocol for KiirooV2Vibrator {
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
}

impl ButtplugProtocolCommandHandler for KiirooV2Vibrator {
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
        device
          .write_value(DeviceWriteCmd::new(
            Endpoint::Tx,
            vec![
              cmds.get(0).unwrap_or(&None).unwrap_or(0) as u8,
              cmds.get(1).unwrap_or(&None).unwrap_or(0) as u8,
              cmds.get(2).unwrap_or(&None).unwrap_or(0) as u8,
            ],
            false,
          ))
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
  pub fn test_kiiroov2vibrator_protocol_3_features() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Titan").await.unwrap();
      let command_receiver = test_device
        .get_endpoint_channel(&Endpoint::Tx)
        .unwrap()
        .receiver;
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.25),
              VibrateSubcommand::new(1, 0.5),
              VibrateSubcommand::new(2, 0.75),
            ],
          )
          .into(),
        )
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![25, 50, 75], false)),
      )
      .await;
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.25),
              VibrateSubcommand::new(1, 0.5),
              VibrateSubcommand::new(2, 0.75),
            ],
          )
          .into(),
        )
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
          vec![0x0, 0x0, 0x0],
          false,
        )),
      )
      .await;
    });
  }

  #[test]
  pub fn test_kiiroov2vibrator_protocol_2_features() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Fuse").await.unwrap();
      let command_receiver = test_device
        .get_endpoint_channel(&Endpoint::Tx)
        .unwrap()
        .receiver;
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.25),
              VibrateSubcommand::new(1, 0.5),
            ],
          )
          .into(),
        )
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![25, 50, 0], false)),
      )
      .await;
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.25),
              VibrateSubcommand::new(1, 0.5),
            ],
          )
          .into(),
        )
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
          vec![0x0, 0x0, 0x0],
          false,
        )),
      )
      .await;
    });
  }

  #[test]
  pub fn test_kiiroov2vibrator_protocol_1_features() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Pearl2").await.unwrap();
      let command_receiver = test_device
        .get_endpoint_channel(&Endpoint::Tx)
        .unwrap()
        .receiver;
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.25)]).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![25, 0, 0], false)),
      )
      .await;
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.25)]).into())
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
          vec![0x0, 0x0, 0x0],
          false,
        )),
      )
      .await;
    });
  }
}
