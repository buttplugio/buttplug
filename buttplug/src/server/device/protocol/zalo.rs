// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(Zalo, "zalo");

#[derive(Default)]
pub struct Zalo {}

impl ProtocolHandler for Zalo {
  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // Store off result before the match, so we drop the lock ASAP.
    let speed0: u8 = cmds[0].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8;
    let speed1: u8 = if cmds.len() == 1 {
      0
    } else {
      cmds[1].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8
    };
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        if speed0 == 0 && speed1 == 0 {
          0x02
        } else {
          0x01
        },
        if speed0 == 0 { 0x01 } else { speed0 },
        if speed1 == 0 { 0x01 } else { speed1 },
      ],
      true,
    )
    .into()])
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
  pub fn test_zalo_protocol_1vibe() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("ZALO-Jeanne")
        .await
        .expect("Test, assuming infallible");
      let command_receiver_tx = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x04, 0x01],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));

      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver_tx));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 1.0)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x08, 0x01],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x02, 0x01, 0x01],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
    });
  }

  #[test]
  pub fn test_zalo_protocol_2vibe() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("ZALO-Queen")
        .await
        .expect("Test, assuming infallible");
      let command_receiver_tx = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.5),
              VibrateSubcommand::new(1, 0.5),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x04, 0x04],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));

      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(1, 1.0)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x04, 0x08],
          true,
        )),
      );

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver_tx));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x02, 0x01, 0x01],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
    });
  }
}
 */
