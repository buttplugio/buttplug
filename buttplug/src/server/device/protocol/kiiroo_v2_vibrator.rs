// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    messages::{ActuatorType, Endpoint},
  },
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(KiirooV2Vibrator, "kiiroov2vibrator");

#[derive(Default)]
pub struct KiirooV2Vibrator {}

impl ProtocolHandler for KiirooV2Vibrator {
  fn handle_scalar_cmd(
    &self,
    cmds: &Vec<Option<(ActuatorType, u32)>>,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        cmds
          .get(0)
          .unwrap_or(&None)
          .unwrap_or((ActuatorType::Vibrate, 0))
          .1 as u8,
        cmds
          .get(1)
          .unwrap_or(&None)
          .unwrap_or((ActuatorType::Vibrate, 0))
          .1 as u8,
        cmds
          .get(2)
          .unwrap_or(&None)
          .unwrap_or((ActuatorType::Vibrate, 0))
          .1 as u8,
      ],
      false,
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
  pub fn test_kiiroov2vibrator_protocol_3_features() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Titan")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.25),
              VibrateSubcommand::new(1, 0.5),
              VibrateSubcommand::new(2, 0.75),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![25, 50, 75], false)),
      );
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.25),
              VibrateSubcommand::new(1, 0.5),
              VibrateSubcommand::new(2, 0.75),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver));
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x0, 0x0, 0x0],
          false,
        )),
      );
    });
  }

  #[test]
  pub fn test_kiiroov2vibrator_protocol_2_features() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Fuse")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.25),
              VibrateSubcommand::new(1, 0.5),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![25, 50, 0], false)),
      );
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.25),
              VibrateSubcommand::new(1, 0.5),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver));
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x0, 0x0, 0x0],
          false,
        )),
      );
    });
  }

  #[test]
  pub fn test_kiiroov2vibrator_protocol_1_features() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Pearl2")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.25)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![25, 0, 0], false)),
      );
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.25)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver));
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x0, 0x0, 0x0],
          false,
        )),
      );
    });
  }
}
 */
