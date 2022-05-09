// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{
    self,
    ButtplugDeviceCommandMessageUnion,
    ButtplugDeviceMessage,
    VibrateCmd,
    VibrateSubcommand,
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    configuration_manager::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder, ProtocolAttributesIdentifier},
    DeviceImpl,
    DeviceReadCmd,
    DeviceWriteCmd,
    Endpoint,
  },
};
use std::sync::Arc;

super::default_protocol_definition!(Vibratissimo, "vibratissimo");

#[derive(Default, Debug)]
pub struct VibratissimoFactory {}

impl ButtplugProtocolFactory for VibratissimoFactory {
  fn try_create(
    &self,
    device_impl: Arc<crate::device::DeviceImpl>,
    builder: ProtocolDeviceAttributesBuilder,
  ) -> futures::future::BoxFuture<
    'static,
    Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>,
  > {
    Box::pin(async move {
      let result = device_impl
        .read_value(DeviceReadCmd::new(Endpoint::RxBLEModel, 128, 500))
        .await?;
      let ident =
        String::from_utf8(result.data().to_vec()).unwrap_or_else(|_| device_impl.name.clone());
      let device_attributes = builder.create(device_impl.address(), &ProtocolAttributesIdentifier::Identifier(ident), &device_impl.endpoints())?;
      Ok(Box::new(Vibratissimo::new(device_attributes)) as Box<dyn ButtplugProtocol>)
    })
  }

  fn protocol_identifier(&self) -> &'static str {
    Vibratissimo::PROTOCOL_IDENTIFIER
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
        for cmd in cmds {
          data.push(cmd.unwrap_or(0) as u8);
        }
        if data.len() == 1 {
          data.push(0x00);
        }

        // Put the device in write mode
        fut_vec.push(device.write_value(DeviceWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )));
        fut_vec.push(device.write_value(DeviceWriteCmd::new(Endpoint::TxVibrate, data, false)));
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
    server::comm_managers::test::{
      check_test_recv_empty,
      check_test_recv_value,
      new_bluetoothle_test_device,
    },
    util::async_manager,
  };

  #[test]
  pub fn test_vibratissimo_protocol_default() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Vibratissimo")
        .await
        .expect("Test, assuming infallible");
      let command_receiver_vibrate = test_device
        .endpoint_receiver(&Endpoint::TxVibrate)
        .expect("Test, assuming infallible");
      let command_receiver_mode = test_device
        .endpoint_receiver(&Endpoint::TxMode)
        .expect("Test, assuming infallible");

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
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
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver_mode));
      assert!(check_test_recv_empty(&command_receiver_vibrate));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
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
      let (device, test_device) = new_bluetoothle_test_device("Vibratissimo")
        .await
        .expect("Test, assuming infallible");
      let command_receiver_vibrate = test_device
        .endpoint_receiver(&Endpoint::TxVibrate)
        .expect("Test, assuming infallible");
      let command_receiver_mode = test_device
        .endpoint_receiver(&Endpoint::TxMode)
        .expect("Test, assuming infallible");

      assert!(check_test_recv_empty(&command_receiver_mode));
      assert!(check_test_recv_empty(&command_receiver_vibrate));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
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
        .expect("Test, assuming infallible");
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
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver_mode));
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");

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
      let (device, test_device) = new_bluetoothle_test_device("Vibratissimo")
        .await
        .expect("Test, assuming infallible");
      let command_receiver_vibrate = test_device
        .endpoint_receiver(&Endpoint::TxVibrate)
        .expect("Test, assuming infallible");
      let command_receiver_mode = test_device
        .endpoint_receiver(&Endpoint::TxMode)
        .expect("Test, assuming infallible");

      assert!(check_test_recv_empty(&command_receiver_mode));
      assert!(check_test_recv_empty(&command_receiver_vibrate));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
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
        .expect("Test, assuming infallible");
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
        .expect("Test, assuming infallible");
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
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver_mode));
      assert!(check_test_recv_empty(&command_receiver_vibrate));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
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
