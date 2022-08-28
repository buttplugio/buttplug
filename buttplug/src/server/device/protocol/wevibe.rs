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

generic_protocol_initializer_setup!(WeVibe, "wevibe");

#[derive(Default)]
pub struct WeVibeInitializer {}

#[async_trait]
impl ProtocolInitializer for WeVibeInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    debug!("calling WeVibe init");
    hardware
      .write_value(&HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![0x0f, 0x03, 0x00, 0x99, 0x00, 0x03, 0x00, 0x00],
        true,
      ))
      .await?;
    hardware
      .write_value(&HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        true,
      ))
      .await?;
    Ok(Arc::new(WeVibe::default()))
  }
}

#[derive(Default)]
pub struct WeVibe {}

impl ProtocolHandler for WeVibe {
  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>]
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let r_speed_int = cmds[0].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8;
    let r_speed_ext = cmds
      .last()
      .unwrap_or(&None)
      .unwrap_or((ActuatorType::Vibrate, 0u32))
      .1 as u8;
    let data = if r_speed_int == 0 && r_speed_ext == 0 {
      vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    } else {
      vec![
        0x0f,
        0x03,
        0x00,
        r_speed_ext | (r_speed_int << 4),
        0x00,
        0x03,
        0x00,
        0x00,
      ]
    };
    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, true).into()])
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
  pub fn test_wevibe_protocol_two_features() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("4 Plus")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x0f, 0x03, 0x00, 0x99, 0x00, 0x03, 0x00, 0x00],
          true,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
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
          vec![0x0f, 0x03, 0x00, 0x80, 0x00, 0x03, 0x00, 0x00],
          true,
        )),
      );
      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver));
      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.25),
              VibrateSubcommand::new(1, 0.75),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      // TODO There's probably a more concise way to do this.
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x0f, 0x03, 0x00, 0x4c, 0x00, 0x03, 0x00, 0x00],
          true,
        )),
      );
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
          true,
        )),
      );
    });
  }

  #[test]
  pub fn test_wevibe_protocol_one_feature() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Ditto")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x0f, 0x03, 0x00, 0x99, 0x00, 0x03, 0x00, 0x00],
          true,
        )),
      );
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
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
          vec![0x0f, 0x03, 0x00, 0x88, 0x00, 0x03, 0x00, 0x00],
          true,
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
          vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
          true,
        )),
      );
    });
  }
}
*/
