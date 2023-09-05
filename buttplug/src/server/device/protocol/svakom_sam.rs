// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::hardware::HardwareSubscribeCmd;
use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::device::{
    configuration::{ProtocolAttributesType, ProtocolDeviceAttributes},
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

generic_protocol_initializer_setup!(SvakomSam, "svakom-sam");

#[derive(Default)]
pub struct SvakomSamInitializer {}

#[async_trait]
impl ProtocolInitializer for SvakomSamInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    hardware
      .subscribe(&HardwareSubscribeCmd::new(Endpoint::Rx))
      .await?;
    Ok(Arc::new(SvakomSam::default()))
  }
}

#[derive(Default)]
pub struct SvakomSam {}

impl ProtocolHandler for SvakomSam {
  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut msg_vec = vec![];
    if let Some((_, speed)) = cmds[0] {
      msg_vec.push(
        HardwareWriteCmd::new(Endpoint::Tx, [18, 1, 3, 0, 5, speed as u8].to_vec(), false).into(),
      );
    }
    if cmds.len() > 1 {
      if let Some((_, speed)) = cmds[1] {
        msg_vec.push(
          HardwareWriteCmd::new(Endpoint::Tx, [18, 6, 1, speed as u8].to_vec(), false).into(),
        );
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
  pub fn test_svakom_sam_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Sam Neo")
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
          vec![18, 1, 3, 0, 5, 5],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));
      // Since we only created one changed subcommand, we should only receive one command.
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
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![18, 6, 1, 1], true)),
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
          vec![18, 1, 3, 0, 5, 0],
          true,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![18, 6, 1, 0], true)),
      );
      assert!(check_test_recv_empty(&command_receiver));
    });
  }
}
 */
