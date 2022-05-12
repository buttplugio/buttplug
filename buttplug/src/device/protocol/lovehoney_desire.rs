// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    configuration_manager::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use std::sync::Arc;

super::default_protocol_declaration!(LovehoneyDesire, "lovehoney-desire");

impl ButtplugProtocolCommandHandler for LovehoneyDesire {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // Store off result before the match, so we drop the lock ASAP.
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, false)?;
      if let Some(cmds) = result {
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
        let mut fut_vec = vec![];
        if cmds[0].is_some() && cmds.windows(2).all(|w| w[0] == w[1]) {
          let fut = device.write_value(DeviceWriteCmd::new(
            Endpoint::Tx,
            vec![
              0xF3,
              0,
              cmds[0].expect("Already checked value existence") as u8,
            ],
            false,
          ));
          fut.await?;
        } else {
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
          for fut in fut_vec {
            fut.await?;
          }
        }
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
    server::device::communication_manager::test::{
      check_test_recv_empty,
      check_test_recv_value,
      new_bluetoothle_test_device,
    },
    util::async_manager,
  };

  #[test]
  pub fn test_lovehoney_desire_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("PROSTATE VIBE")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");

      // If we send one speed to one motor, we should only see one output.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0xF3, 0x1, 0x40],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));

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
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0xF3, 0x0, 0x0d],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));

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
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0xF3, 0x01, 0x00],
          false,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0xF3, 0x02, 0x40],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0xF3, 0x02, 0x0],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));
    });
  }
}
