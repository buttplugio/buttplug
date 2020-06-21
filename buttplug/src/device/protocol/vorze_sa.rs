use super::{ButtplugProtocol, ButtplugProtocolCommandHandler, ButtplugProtocolCreator};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, MessageAttributesMap},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl, DeviceWriteCmd, Endpoint,
  },
  server::ButtplugServerResultFuture,
};
use async_mutex::Mutex;
use std::sync::Arc;

#[derive(ButtplugProtocol, ButtplugProtocolCreator, ButtplugProtocolProperties)]
pub struct VorzeSA {
  name: String,
  message_attributes: MessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl VorzeSA {
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

#[repr(u8)]
enum VorzeDevices {
  Bach = 6,
  UFO = 2,
  Cyclone = 1,
}

#[repr(u8)]
enum VorzeActions {
  Rotate = 1,
  Vibrate = 3,
}

impl ButtplugProtocolCommandHandler for VorzeSA {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
    msg: messages::VibrateCmd,
  ) -> ButtplugServerResultFuture {
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&msg, false);
      let mut fut_vec = vec![];
      match result {
        Ok(cmds_option) => {
          if let Some(cmds) = cmds_option {
            if let Some(speed) = cmds[0] {
              fut_vec.push(
                device.write_value(
                  DeviceWriteCmd::new(
                    Endpoint::Tx,
                    vec![
                      VorzeDevices::Bach as u8,
                      VorzeActions::Vibrate as u8,
                      speed as u8,
                    ],
                    false,
                  )
                  .into(),
                ),
              );
            }
          }

          for fut in fut_vec {
            fut.await?;
          }
          Ok(messages::Ok::default().into())
        }
        Err(e) => Err(e.into()),
      }
    })
  }

  fn handle_rotate_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
    msg: messages::RotateCmd,
  ) -> ButtplugServerResultFuture {
    let manager = self.manager.clone();
    // This will never change, so we can process it before the future. 
    let dev_id = if self.name.contains("UFO") {
      VorzeDevices::UFO
    } else {
      VorzeDevices::Cyclone
    };
    Box::pin(async move {
      let result = manager.lock().await.update_rotation(&msg);
      let mut fut_vec = vec![];
      match result {
        Ok(cmds) => {
          if let Some((speed, clockwise)) = cmds[0] {
            let data: u8 = (clockwise as u8) << 7 | (speed as u8);
            fut_vec.push(
              device.write_value(
                DeviceWriteCmd::new(
                  Endpoint::Tx,
                  vec![dev_id as u8, VorzeActions::Rotate as u8, data],
                  false,
                )
                .into(),
              ),
            );
          }

          for fut in fut_vec {
            fut.await?;
          }
          Ok(messages::Ok::default().into())
        }
        Err(e) => Err(e.into()),
      }
    })
  }
}

#[cfg(test)]
mod test {
  use crate::{
    core::messages::{RotateCmd, RotationSubcommand, StopDeviceCmd, VibrateCmd, VibrateSubcommand},
    device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
    test::{check_recv_value, new_bluetoothle_test_device},
    util::async_manager,
  };

  #[test]
  pub fn test_vorze_sa_vibration_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Bach smart").await.unwrap();
      let command_receiver = test_device
        .get_endpoint_channel(&Endpoint::Tx)
        .unwrap()
        .receiver;
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x06, 0x03, 50],
          false,
        )),
      )
      .await;
      assert!(command_receiver.is_empty());

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x06, 0x03, 0x0],
          false,
        )),
      )
      .await;
      assert!(command_receiver.is_empty());
    });
  }

  #[test]
  pub fn test_vorze_sa_rotation_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("CycSA").await.unwrap();
      let command_receiver = test_device
        .get_endpoint_channel(&Endpoint::Tx)
        .unwrap()
        .receiver;
      device
        .parse_message(RotateCmd::new(0, vec![RotationSubcommand::new(0, 0.5, false)]).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x01, 49],
          false,
        )),
      )
      .await;
      assert!(command_receiver.is_empty());

      device
        .parse_message(RotateCmd::new(0, vec![RotationSubcommand::new(0, 0.5, true)]).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x01, 177],
          false,
        )),
      )
      .await;
      assert!(command_receiver.is_empty());

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x01, 0x0],
          false,
        )),
      )
      .await;
      assert!(command_receiver.is_empty());
    });
  }
}
