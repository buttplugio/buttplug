// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    configuration::ProtocolAttributesType,
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
    ServerDeviceIdentifier,
  },
};
use async_trait::async_trait;
use std::sync::Arc;
use crate::server::device::configuration::ProtocolDeviceAttributes;

generic_protocol_initializer_setup!(LoveDistance, "lovedistance");

#[derive(Default)]
pub struct LoveDistanceInitializer {}

#[async_trait]
impl ProtocolInitializer for LoveDistanceInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let msg = HardwareWriteCmd::new(Endpoint::Tx, vec![0xf3, 0, 0], false);
    hardware.write_value(&msg).await?;
    let msg = HardwareWriteCmd::new(Endpoint::Tx, vec![0xf4, 1], false);
    hardware.write_value(&msg).await?;
    Ok(Arc::new(LoveDistance::default()))
  }
}

#[derive(Default)]
pub struct LoveDistance {}

impl ProtocolHandler for LoveDistance {
  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0xf3, 0x00, scalar as u8],
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
      hardware::communication::test::{check_test_recv_value, new_bluetoothle_test_device},
      hardware::{HardwareCommand, HardwareWriteCmd},
    },
    util::async_manager,
  };

  #[test]
  pub fn test_lovedistance_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("REACH G")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xf3, 0, 0], false)),
      );
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xf4, 01], false)),
      );

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0xf3, 0, 0x3d],
          false,
        )),
      );
      // Test to make sure we handle packet IDs across protocol clones correctly.
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xf3, 0, 0], false)),
      );
    });
  }
}
*/
