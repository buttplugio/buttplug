// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::handle_nonaggregate_vibrate_cmd;
use crate::{
  core::{errors::ButtplugDeviceError, messages::Endpoint},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(LiboElle, "libo-elle");

#[derive(Default)]
pub struct LiboElle {}

impl ProtocolHandler for LiboElle {
  fn handle_vibrate_cmd(
    &self,
    cmds: &Vec<Option<u32>>
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(handle_nonaggregate_vibrate_cmd(cmds, |index, speed| {
      if index == 1 {
        let mut data = 0u8;
        if speed as u8 > 0 && speed as u8 <= 7 {
          data |= (speed as u8 - 1) << 4;
          data |= 1; // Set the mode too
        } else if speed as u8 > 7 {
          data |= (speed as u8 - 8) << 4;
          data |= 4; // Set the mode too
        }
        HardwareWriteCmd::new(Endpoint::Tx, vec![data], false).into()
      } else {
        HardwareWriteCmd::new(
          Endpoint::TxMode,
          vec![speed as u8],
          false,
        ).into()
      }
    }))
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
  pub fn test_libo_elle_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("PiPiJing")
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
        &command_receiver_tx_mode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x02], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx_mode));
      assert!(check_test_recv_empty(&command_receiver_tx));

      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 1.0)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx_mode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x03], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx_mode));
      assert!(check_test_recv_empty(&command_receiver_tx));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 1.0)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_tx_mode));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_tx_mode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x00], false)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx_mode));
      assert!(check_test_recv_empty(&command_receiver_tx));
    });
  }
}
 */
