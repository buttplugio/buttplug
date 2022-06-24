// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{
  fleshlight_launch_helper::calculate_speed,
  handle_nonaggregate_vibrate_cmd
};
use crate::{
  core::{errors::ButtplugDeviceError, messages::{self, Endpoint, ButtplugDeviceMessage}},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};
use std::sync::{Arc, atomic::{AtomicU8, Ordering::SeqCst}};

generic_protocol_setup!(KiirooV21, "kiiroo-v21");

#[derive(Default)]
pub struct KiirooV21 {
  previous_position: Arc<AtomicU8>
}

impl ProtocolHandler for KiirooV21 {
  fn handle_vibrate_cmd(
    &self,
    cmds: &Vec<Option<u32>>
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(handle_nonaggregate_vibrate_cmd(cmds, |_, speed| HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0x01, speed as u8],
      false,
    ).into()))
  }

  fn handle_linear_cmd(
    &self,
    message: messages::LinearCmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let v = message.vectors()[0].clone();
    // In the protocol, we know max speed is 99, so convert here. We have to
    // use AtomicU8 because there's no AtomicF64 yet.
    let previous_position = self.previous_position.load(SeqCst);
    let distance = (previous_position as f64 - (v.position * 99f64)).abs() / 99f64;
    let fl_cmd = messages::FleshlightLaunchFW12Cmd::new(
      message.device_index(),
      (v.position * 99f64) as u8,
      (calculate_speed(distance, v.duration) * 99f64) as u8,
    );
    self.handle_fleshlight_launch_fw12_cmd(fl_cmd)
  }

  fn handle_fleshlight_launch_fw12_cmd(
    &self,
    message: messages::FleshlightLaunchFW12Cmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let previous_position = self.previous_position.clone();
    let position = message.position();
    previous_position.store(position, SeqCst);
    Ok(vec!(HardwareWriteCmd::new(
      Endpoint::Tx,
      [0x03, 0x00, message.speed(), message.position()].to_vec(),
      false).into()))
  }
}

/*
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
      hardware::communication::test::{
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
 */