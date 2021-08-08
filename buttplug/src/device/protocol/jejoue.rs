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
pub struct JeJoue {
  name: String,
  message_attributes: DeviceMessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl ButtplugProtocol for JeJoue {
  fn new_protocol(
    name: &str,
    message_attributes: DeviceMessageAttributesMap,
  ) -> Box<dyn ButtplugProtocol>
  where
    Self: Sized,
  {
    let manager = GenericCommandManager::new(&message_attributes);

    Box::new(Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
    })
  }
}

impl ButtplugProtocolCommandHandler for JeJoue {
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
        // Default to both vibes
        let mut pattern: u8 = 1;

        // Use vibe 1 as speed
        let mut speed = cmds[0].unwrap_or(0) as u8;

        // Unless it's zero, then five vibe 2 a chance
        if speed == 0 {
          speed = cmds[1].unwrap_or(0) as u8;

          // If we've vibing on 2 only, then change the pattern
          if speed != 0 {
            pattern = 3;
          }
        }

        // If we've vibing on 1 only, then change the pattern
        if pattern == 1 && speed != 0 && cmds[1].unwrap_or(0) == 0 {
          pattern = 2;
        }

        device
          .write_value(DeviceWriteCmd::new(
            Endpoint::Tx,
            vec![pattern, speed],
            false,
          ))
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
    test::{check_test_recv_empty, check_test_recv_value, new_bluetoothle_test_device},
    util::async_manager,
  };

  #[test]
  pub fn test_je_joue_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Je Joue").await.unwrap();
      let command_receiver = test_device.get_endpoint_receiver(&Endpoint::Tx).unwrap();
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
      // We just vibe 1 so expect 1 write (mode 2)
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![2, 3], false)),
      );
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
      // no-op
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.1),
              VibrateSubcommand::new(1, 0.5),
            ],
          )
          .into(),
        )
        .await
        .unwrap();
      // setting second vibe whilst changing vibe 1, 1 writes (mode 1)
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![1, 1], false)),
      );
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.1),
              VibrateSubcommand::new(1, 0.9),
            ],
          )
          .into(),
        )
        .await
        .unwrap();
      // only vibe 1 changed, 1 write, same data
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![1, 1], false)),
      );
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.0),
              VibrateSubcommand::new(1, 0.9),
            ],
          )
          .into(),
        )
        .await
        .unwrap();
      // turn off vibe 1, 1 write (mode 3)
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![3, 5], false)),
      );
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .unwrap();
      // stop on both, 1 write (mode 1)
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![1, 0], false)),
      );
      assert!(check_test_recv_empty(&command_receiver));
    });
  }
}
