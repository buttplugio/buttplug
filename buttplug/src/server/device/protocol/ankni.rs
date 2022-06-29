// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    messages::Endpoint,
  },
  server::device::{
    configuration::ProtocolAttributesType,
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer, generic_protocol_initializer_setup},
    ServerDeviceIdentifier,
  },
};
use async_trait::async_trait;
use std::{
  sync::{
    Arc,
  },
};

generic_protocol_initializer_setup!(Ankni, "ankni");

#[derive(Default)]
pub struct AnkniInitializer {}

#[async_trait]
impl ProtocolInitializer for AnkniInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<Box<dyn ProtocolHandler>, ButtplugDeviceError> {
    let msg = HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01, 0x01, 0x01, 0x01, 0x01,
      ],
      true,
    );
    hardware.write_value(&msg).await?;
    let msg = HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        0x01, 0x02, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd,
        0xfd, 0xfd, 0xfd, 0x00, 0x00,
      ],
      true,
    );
    hardware.write_value(&msg).await?;
    Ok(Box::new(Ankni::default()))
  }
}

#[derive(Default)]
pub struct Ankni {}

impl ProtocolHandler for Ankni {
  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec!(HardwareWriteCmd::new(
              Endpoint::Tx,
              vec![
                0x03,
                0x12,
                scalar as u8,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
              ],
              true,
            ).into()))
        }
      }

/*
#[cfg(all(test, feature = "server"))]
mod test {
  use crate::{
    core::messages::{Endpoint, StopDeviceCmd, VibrateCmd, VibrateSubcommand},
    server::device::{
      hardware::communication::test::{check_test_recv_value, new_bluetoothle_test_device},
      hardware::{HardwareCommand, HardwareWriteCmd},
    },
    util::async_manager,
  };

  #[test]
  pub fn test_ankni_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("DSJM")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![
            0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
            0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
          ],
          true,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![
            0x01, 0x02, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd,
            0xfd, 0xfd, 0xfd, 0xfd, 0x00, 0x00,
          ],
          true,
        )),
      );

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![
            0x03, 0x12, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
          ],
          true,
        )),
      );
      // Test to make sure we handle packet IDs across protocol clones correctly.
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![
            0x03, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
          ],
          true,
        )),
      );
    });
  }
}
*/