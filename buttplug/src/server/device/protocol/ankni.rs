// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, Endpoint},
  server::{
    ButtplugServerResultFuture,
    device::{
      protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
      configuration::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder},
      hardware::{Hardware, HardwareWriteCmd},
    },
  }
};
use std::sync::Arc;

super::default_protocol_definition!(Ankni, "ankni");

#[derive(Default, Debug)]
pub struct AnkniFactory {}

impl ButtplugProtocolFactory for AnkniFactory {
  fn try_create(
    &self,
    hardware: Arc<Hardware>,
    builder: ProtocolDeviceAttributesBuilder,
  ) -> futures::future::BoxFuture<
    'static,
    Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>,
  > {
    Box::pin(async move {
      let msg = HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![
          0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
          0x01, 0x01, 0x01, 0x01, 0x01,
        ],
        true,
      );
      hardware.write_value(msg).await?;
      let msg = HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![
          0x01, 0x02, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd, 0xfd,
          0xfd, 0xfd, 0xfd, 0x00, 0x00,
        ],
        true,
      );
      hardware.write_value(msg).await?;
      let device_attributes = builder.create_from_hardware(&hardware)?;
      Ok(Box::new(Ankni::new(device_attributes)) as Box<dyn ButtplugProtocol>)
    })
  }

  fn protocol_identifier(&self,) -> &'static str {
    "ankni"
  }
}

impl ProtocolHandler for Ankni {
  fn handle_vibrate_cmd(
    &self,
    cmds: &Vec<Option<u32>>,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, false)?;
      if let Some(cmds) = result {
        if let Some(speed) = cmds[0] {
          device
            .write_value(HardwareWriteCmd::new(
              Endpoint::Tx,
              vec![
                0x03,
                0x12,
                speed as u8,
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
            ))
            .await?;
        }
      }

      Ok(messages::Ok::default().into())
    })
  }
}

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
