use super::{ButtplugProtocol, ButtplugProtocolCommandHandler, ButtplugProtocolCreator};
use crate::{
  core::{
    messages::{self, ButtplugDeviceCommandMessageUnion, MessageAttributesMap},
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl, DeviceWriteCmd, Endpoint,
  },
  server::ButtplugServerResultFuture,
};
use std::cell::RefCell;

#[derive(ButtplugProtocol, ButtplugProtocolCreator, ButtplugProtocolProperties)]
pub struct LovehoneyDesire {
  name: String,
  message_attributes: MessageAttributesMap,
  manager: RefCell<GenericCommandManager>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl LovehoneyDesire {
  pub(super) fn new(name: &str, message_attributes: MessageAttributesMap) -> Self {
    let manager = GenericCommandManager::new(&message_attributes);

    Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      manager: RefCell::new(manager),
    }
  }
}

impl ButtplugProtocolCommandHandler for LovehoneyDesire {
  fn handle_vibrate_cmd(
    &self,
    device: &dyn DeviceImpl,
    message: messages::VibrateCmd,
  ) -> ButtplugServerResultFuture {
    // Store off result before the match, so we drop the lock ASAP.
    let result = self.manager.borrow_mut().update_vibration(&message, false);
    match result {
      Ok(cmds_option) => {
        let mut fut_vec = vec![];
        if let Some(cmds) = cmds_option {
          // The Lovehoney Desire has 2 types of commands
          //
          // - Set both motors with one command
          // - Set each motor separately
          //
          // We'll need to check what we got back and write our
          // commands accordingly.
          //
          // Neat way of checking if everything is the same via
          // https://sts10.github.io/2019/06/06/is-all-equal-function.html.
          //
          // Just make sure we're not matching on None, 'cause if
          // that's the case we ain't got shit to do.
          if !cmds[0].is_none() && cmds.windows(2).all(|w| w[0] == w[1]) {
            let fut = device.write_value(DeviceWriteCmd::new(
              Endpoint::Tx,
              vec![0xF3, 0, cmds[0].unwrap() as u8],
              false,
            ));
            return Box::pin(async move {
              fut.await?;
              Ok(messages::Ok::default().into())
            });
          }
          // We have differening values. Set each motor separately.
          let mut i = 1;

          for cmd in cmds {
            if let Some(speed) = cmd {
              fut_vec.push(device.write_value(DeviceWriteCmd::new(
                Endpoint::Tx,
                vec![0xF3, i, speed as u8],
                false,
              )));
            }
            i += 1;
          }
        }
        Box::pin(async move {
          for fut in fut_vec {
            fut.await?;
          }
          Ok(messages::Ok::default().into())
        })
      }
      Err(e) => e.into(),
    }
  }
}

#[cfg(test)]
mod test {
  use crate::{
    core::messages::{StopDeviceCmd, VibrateCmd, VibrateSubcommand},
    device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
    test::{check_recv_value, TestDevice},
    util::async_manager,
  };

  #[test]
  pub fn test_lovehoney_desire_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = TestDevice::new_bluetoothle_test_device("PROSTATE VIBE")
        .await
        .unwrap();
      let (_, command_receiver) = test_device.get_endpoint_channel_clone(Endpoint::Tx).await;

      // If we send one speed to one motor, we should only see one output.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0xF3, 0x1, 0x3f],
          false,
        )),
      )
      .await;
      assert!(command_receiver.is_empty());

      // If we send the same speed to each motor, we should only get one command.
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.1),
              VibrateSubcommand::new(1, 0.1),
            ],
          )
          .into(),
        )
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0xF3, 0x0, 0x0c],
          false,
        )),
      )
      .await;
      assert!(command_receiver.is_empty());

      // If we send different commands to both motors, we should get 2 different commands, each with an index.
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.0),
              VibrateSubcommand::new(1, 0.5),
            ],
          )
          .into(),
        )
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0xF3, 0x01, 0x00],
          false,
        )),
      )
      .await;
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0xF3, 0x02, 0x3f],
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
          vec![0xF3, 0x02, 0x0],
          false,
        )),
      )
      .await;
      assert!(command_receiver.is_empty());
    });
  }
}
