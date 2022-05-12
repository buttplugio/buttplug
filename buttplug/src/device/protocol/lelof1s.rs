// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler};
use crate::device::DeviceSubscribeCmd;
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

super::default_protocol_definition!(LeloF1s, "lelof1s");

#[derive(Default, Debug)]
pub struct LeloF1sFactory {}

impl ButtplugProtocolFactory for LeloF1sFactory {
  fn try_create(
    &self,
    device_impl: Arc<crate::device::DeviceImpl>,
    builder: ProtocolDeviceAttributesBuilder,
  ) -> futures::future::BoxFuture<
    'static,
    Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>,
  > {
    // The Lelo F1s needs you to hit the power button after connection
    // before it'll accept any commands. Unless we listen for event on
    // the button, this is more likely to turn the device off.
    let subscribe_fut = device_impl.subscribe(DeviceSubscribeCmd::new(Endpoint::Rx));
    Box::pin(async move {
      subscribe_fut.await?;
      let device_attributes = builder.create_from_device_impl(&device_impl)?;
      Ok(Box::new(LeloF1s::new(device_attributes)) as Box<dyn ButtplugProtocol>)
    })
  }

  fn protocol_identifier(&self) -> &'static str {
    "lelo-f1s"
  }
}

impl ButtplugProtocolCommandHandler for LeloF1s {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // Store off result before the match, so we drop the lock ASAP.
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, true)?;
      let mut cmd_vec = vec![0x1];
      if let Some(cmds) = result {
        info!("{:?}", cmds);
        for cmd in cmds.iter() {
          cmd_vec.push(cmd.expect("Test, assuming infallible") as u8);
        }
        device
          .write_value(DeviceWriteCmd::new(Endpoint::Tx, cmd_vec, false))
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
    server::device::communication_manager::test::{
      check_test_recv_empty,
      check_test_recv_value,
      new_bluetoothle_test_device,
    },
    util::async_manager,
  };

  #[test]
  pub fn test_lelof1s_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("F1s")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x32, 0x0],
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
        .expect("Test, assuming infallible");
      // TODO There's probably a more concise way to do this.
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x1, 0xa, 0x32],
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
          vec![0x1, 0x0, 0x0],
          false,
        )),
      );
    });
  }
}
