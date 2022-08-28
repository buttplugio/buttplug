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

generic_protocol_setup!(LiboVibes, "libo-vibes");

#[derive(Default)]
pub struct LiboVibes {}

impl ProtocolHandler for LiboVibes {
  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut msg_vec = vec![];
    for (index, cmd) in cmds.iter().enumerate() {
      if let Some((_, speed)) = cmd {
        if index == 0 {
          msg_vec.push(HardwareWriteCmd::new(Endpoint::Tx, vec![*speed as u8], false).into());

          // If this is a single vibe device, we need to send stop to TxMode too
          if *speed as u8 == 0 && cmds.len() == 1 {
            msg_vec.push(HardwareWriteCmd::new(Endpoint::TxMode, vec![0u8], false).into());
          }
        } else if index == 1 {
          msg_vec.push(HardwareWriteCmd::new(Endpoint::TxMode, vec![*speed as u8], false).into());
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
  pub fn test_libo_vibes_protocol_1vibe() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Yuyi")
        .await
        .expect("Test, assuming infallible");
      let command_receiver_tx = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      let command_receiver_tx_mode = test_device
        .endpoint_receiver(&Endpoint::TxMode)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0x32], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_tx_mode));

      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_tx_mode));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0x00], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      check_test_recv_value(
        &command_receiver_tx_mode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x00], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx_mode));
    });
  }
  #[test]
  pub fn test_libo_vibes_protocol_2vibe() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Gugudai")
        .await
        .expect("Test, assuming infallible");
      let command_receiver_tx = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      let command_receiver_tx_mode = test_device
        .endpoint_receiver(&Endpoint::TxMode)
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
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0x32], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      check_test_recv_value(
        &command_receiver_tx_mode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x02], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx_mode));

      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(1, 1.0)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx_mode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x03], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx_mode));
      assert!(check_test_recv_empty(&command_receiver_tx));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_tx_mode));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0x00], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      check_test_recv_value(
        &command_receiver_tx_mode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x00], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx_mode));
    });
  }
}
 */
