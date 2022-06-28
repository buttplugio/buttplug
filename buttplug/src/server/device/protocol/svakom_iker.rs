// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, messages::{Endpoint, ActuatorType}},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(SvakomIker, "svakom-iker");

#[derive(Default)]
pub struct SvakomIker {}

impl ProtocolHandler for SvakomIker {
  fn handle_scalar_cmd(
    &self,
    cmds: &Vec<Option<(ActuatorType, u32)>>,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut vibe_off = false;
    let mut msg_vec = vec![];
    if let Some((_, speed)) = cmds[0] {
      if speed == 0 {
        vibe_off = true;
      }
      msg_vec.push(
        HardwareWriteCmd::new(
          Endpoint::Tx,
          [0x55, 0x03, 0x03, 0x00, 0x01, speed as u8].to_vec(),
          true,
        )
        .into(),
      );
    }
    if cmds.len() > 1 {
      if let Some((_, speed)) = cmds[1] {
        if speed != 0 || !vibe_off {
          msg_vec.push(
            HardwareWriteCmd::new(
              Endpoint::Tx,
              [0x55, 0x07, 0x00, 0x00, speed as u8, 0x00].to_vec(),
              true,
            )
            .into(),
          );
        }
      } else if vibe_off {
        if let Some((_, speed)) = cmds[1] {
          if speed != 0 {
            msg_vec.push(
              HardwareWriteCmd::new(
                Endpoint::Tx,
                [0x55, 0x07, 0x00, 0x00, speed as u8, 0x00].to_vec(),
                true,
              )
              .into()
            )
          }
        }
      }
    }
    Ok(msg_vec)
  }
}
/*

#[cfg(all(test, feature = "server"))]
mod test {
  use crate::{
    core::messages::{Endpoint, StopDeviceCmd, VibrateCmd, VibrateSubcommand},
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
  pub fn test_svakom_iker_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Iker")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");

      // Turn on the vibe
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      // Test the vibe write
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![85, 3, 3, 0, 1, 5],
          true,
        )),
      );
      // Since we only created one changed subcommand, we should only receive one command.
      assert!(check_test_recv_empty(&command_receiver));

      // Add in the Pulser
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.5),
              VibrateSubcommand::new(1, 1.0),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      // Test the pulser write
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![85, 7, 0, 0, 5, 0],
          true,
        )),
      );
      // Since we only created one changed subcommand, we should only receive one command.
      assert!(check_test_recv_empty(&command_receiver));

      // Stop just the vibe
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.0)]).into())
        .await
        .expect("Test, assuming infallible");
      // Test the vibe write
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![85, 3, 3, 0, 1, 0],
          true,
        )),
      );
      // Test the pulse write
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![85, 7, 0, 0, 5, 0],
          true,
        )),
      );
      // That should be all the commands
      assert!(check_test_recv_empty(&command_receiver));

      // Turn on the vibe
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      // Test the vibe write
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![85, 3, 3, 0, 1, 5],
          true,
        )),
      );
      // Since we only created one changed subcommand, we should only receive one command.
      assert!(check_test_recv_empty(&command_receiver));

      // Stop just the pulser
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(1, 0.0)]).into())
        .await
        .expect("Test, assuming infallible");
      // Test the pulse write
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![85, 7, 0, 0, 0, 0],
          true,
        )),
      );
      // That should be all the commands
      assert!(check_test_recv_empty(&command_receiver));

      // Turn the puler back on
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(1, 1.0)]).into())
        .await
        .expect("Test, assuming infallible");
      // Test the pulser write
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![85, 7, 0, 0, 5, 0],
          true,
        )),
      );
      // Since we only created one changed subcommand, we should only receive one command.
      assert!(check_test_recv_empty(&command_receiver));

      // All stop! Only need to send the vibe stop
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![85, 3, 3, 0, 1, 0],
          true,
        )),
      );
      // That should be all
      assert!(check_test_recv_empty(&command_receiver));
    });
  }
}
 */
