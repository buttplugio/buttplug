// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};

generic_protocol_setup!(Motorbunny, "motorbunny");

#[derive(Default)]
pub struct Motorbunny {}

impl ProtocolHandler for Motorbunny {
  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut command_vec: Vec<u8>;
    if scalar == 0 {
      command_vec = vec![0xf0, 0x00, 0x00, 0x00, 0x00, 0xec];
    } else {
      command_vec = vec![0xff];
      let mut vibe_commands = [scalar as u8, 0x14].repeat(7);
      let crc = vibe_commands
        .iter()
        .fold(0u8, |a, b| a.overflowing_add(*b).0);
      command_vec.append(&mut vibe_commands);
      command_vec.append(&mut vec![crc, 0xec]);
    }
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      command_vec,
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
  pub fn test_motorbunny_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("MB Controller")
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
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![
            0xff, 0x80, 0x14, 0x80, 0x14, 0x80, 0x14, 0x80, 0x14, 0x80, 0x14, 0x80, 0x14, 0x80,
            0x14, 0x0c, 0xec,
          ],
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
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0xf0, 0x00, 0x00, 0x00, 0x00, 0xec],
          false,
        )),
      );
    });
  }
}
 */
