// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{
  fleshlight_launch_helper::calculate_speed,
  ButtplugProtocol,
  ButtplugProtocolFactory,
  ButtplugProtocolCommandHandler,
};
use crate::{
  core::messages::{
    self,
    ButtplugDeviceCommandMessageUnion,
    ButtplugDeviceMessage,
    Endpoint,
    FleshlightLaunchFW12Cmd,
  },
  server::device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    configuration::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder},
    hardware::{Hardware, HardwareWriteCmd, ButtplugDeviceResultFuture},
  },
};
use std::sync::{
  atomic::{AtomicU8, Ordering::SeqCst},
  Arc,
};
use tokio::sync::Mutex;

pub struct KiirooV21 {
  device_attributes: ProtocolDeviceAttributes,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  previous_position: Arc<AtomicU8>,
}

impl KiirooV21 {
  const PROTOCOL_IDENTIFIER: &'static str = "kiiroo-v21";

  fn new(device_attributes: crate::server::device::configuration::ProtocolDeviceAttributes) -> Self {
    let manager = GenericCommandManager::new(&device_attributes);

    Self {
      device_attributes,
      stop_commands: manager.stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
      previous_position: Arc::new(AtomicU8::new(0)),
    }
  }
}

crate::default_protocol_trait_declaration!(KiirooV21);
crate::default_protocol_properties_definition!(KiirooV21);

impl ButtplugProtocol for KiirooV21 {}

impl ButtplugProtocolCommandHandler for KiirooV21 {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // Store off result before the match, so we drop the lock ASAP.
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, false)?;
      if let Some(cmds) = result {
        device
          .write_value(HardwareWriteCmd::new(
            Endpoint::Tx,
            vec![0x01, cmds.get(0).unwrap_or(&None).unwrap_or(0) as u8],
            false,
          ))
          .await?;
      }
      Ok(messages::Ok::default().into())
    })
  }

  fn handle_linear_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::LinearCmd,
  ) -> ButtplugDeviceResultFuture {
    let v = message.vectors()[0].clone();
    // In the protocol, we know max speed is 99, so convert here. We have to
    // use AtomicU8 because there's no AtomicF64 yet.
    let previous_position = self.previous_position.load(SeqCst);
    let distance = (previous_position as f64 - (v.position * 99f64)).abs() / 99f64;
    let fl_cmd = FleshlightLaunchFW12Cmd::new(
      message.device_index(),
      (v.position * 99f64) as u8,
      (calculate_speed(distance, v.duration) * 99f64) as u8,
    );
    self.handle_fleshlight_launch_fw12_cmd(device, fl_cmd)
  }

  fn handle_fleshlight_launch_fw12_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::FleshlightLaunchFW12Cmd,
  ) -> ButtplugDeviceResultFuture {
    let previous_position = self.previous_position.clone();
    let position = message.position();
    let msg = HardwareWriteCmd::new(
      Endpoint::Tx,
      [0x03, 0x00, message.speed(), message.position()].to_vec(),
      false,
    );
    let fut = device.write_value(msg);
    Box::pin(async move {
      previous_position.store(position, SeqCst);
      fut.await?;
      Ok(messages::Ok::default().into())
    })
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use crate::{
    core::messages::{
      Endpoint,      
      FleshlightLaunchFW12Cmd,
      LinearCmd,
      StopDeviceCmd,
      VectorSubcommand,
      VibrateCmd,
      VibrateSubcommand,
    },
    server::device::{
      hardware::{HardwareCommand, HardwareWriteCmd},
      communication::test::{
        check_test_recv_empty,
        check_test_recv_value,
        new_bluetoothle_test_device,
      },
    },
    util::async_manager,
  };

  #[test]
  #[ignore = "None of the linear devices have known issues with initialisation yet"]
  pub fn test_kiiroov21_fleshlight_fw12cmd() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Onyx2.1")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver));
      device
        .parse_message(FleshlightLaunchFW12Cmd::new(0, 50, 50).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x03, 0x00, 50, 50],
          false,
        )),
      );
    });
  }

  #[test]
  #[ignore = "None of the linear devices have known issues with initialisation yet"]
  pub fn test_kiiroov21_linearcmd() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Onyx2.1")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver));
      device
        .parse_message(LinearCmd::new(0, vec![VectorSubcommand::new(0, 500, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x03, 0x00, 19, 49],
          false,
        )),
      );
    });
  }

  #[test]
  pub fn test_kiiroov21_vibratecmd() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Cliona")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver));
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0x01, 50], false)),
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
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0x01, 0], false)),
      );
    });
  }
}
