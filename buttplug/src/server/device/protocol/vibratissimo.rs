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
    configuration::ProtocolAttributesType,
    hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
    protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
    ServerDeviceIdentifier,
  },
};
use async_trait::async_trait;
use std::sync::Arc;

pub mod setup {
  use crate::server::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct VibratissimoIdentifierFactory {}

  impl ProtocolIdentifierFactory for VibratissimoIdentifierFactory {
    fn identifier(&self) -> &str {
      "vibratissimo"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::VibratissimoIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct VibratissimoIdentifier {}

#[async_trait]
impl ProtocolIdentifier for VibratissimoIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<(ServerDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let result = hardware
      .read_value(&HardwareReadCmd::new(Endpoint::RxBLEModel, 128, 500))
      .await?;
    let ident =
      String::from_utf8(result.data().to_vec()).unwrap_or_else(|_| hardware.name().to_owned());
    Ok((
      ServerDeviceIdentifier::new(
        hardware.address(),
        "vibratissimo",
        &ProtocolAttributesType::Identifier(ident),
      ),
      Box::new(VibratissimoInitializer::default()),
    ))
  }
}

#[derive(Default)]
pub struct VibratissimoInitializer {}

#[async_trait]
impl ProtocolInitializer for VibratissimoInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
  ) -> Result<Box<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Box::new(Vibratissimo::default()))
  }
}

#[derive(Default)]
pub struct Vibratissimo {}

impl ProtocolHandler for Vibratissimo {
  fn handle_scalar_cmd(
    &self,
    cmds: &Vec<Option<(ActuatorType, u32)>>,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut data: Vec<u8> = Vec::new();
    for cmd in cmds {
      data.push(cmd.unwrap_or((ActuatorType::Vibrate, 0)).1 as u8);
    }
    if data.len() == 1 {
      data.push(0x00);
    }

    // Put the device in write mode
    Ok(vec![
      HardwareWriteCmd::new(Endpoint::TxMode, vec![0x03, 0xff], false).into(),
      HardwareWriteCmd::new(Endpoint::TxVibrate, data, false).into(),
    ])
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
  pub fn test_vibratissimo_protocol_default() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Vibratissimo")
        .await
        .expect("Test, assuming infallible");
      let command_receiver_vibrate = test_device
        .endpoint_receiver(&Endpoint::TxVibrate)
        .expect("Test, assuming infallible");
      let command_receiver_mode = test_device
        .endpoint_receiver(&Endpoint::TxMode)
        .expect("Test, assuming infallible");

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_vibrate,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x80, 0x00],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));

      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver_mode));
      assert!(check_test_recv_empty(&command_receiver_vibrate));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_vibrate,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x0, 0x0],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));
    });
  }

  #[test]
  #[ignore = "Need to be able to set BLE model info to be read on test device"]
  pub fn test_vibratissimo_protocol_licker() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Vibratissimo")
        .await
        .expect("Test, assuming infallible");
      let command_receiver_vibrate = test_device
        .endpoint_receiver(&Endpoint::TxVibrate)
        .expect("Test, assuming infallible");
      let command_receiver_mode = test_device
        .endpoint_receiver(&Endpoint::TxMode)
        .expect("Test, assuming infallible");

      assert!(check_test_recv_empty(&command_receiver_mode));
      assert!(check_test_recv_empty(&command_receiver_vibrate));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_vibrate,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x80, 0x00],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(1, 1.0)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_vibrate,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x80, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));

      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver_mode));
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");

      check_test_recv_value(
        &command_receiver_vibrate,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x0, 0x0],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));
    });
  }

  #[test]
  #[ignore = "Need to be able to set BLE model info to be read on test device"]
  pub fn test_vibratissimo_protocol_rabbit() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Vibratissimo")
        .await
        .expect("Test, assuming infallible");
      let command_receiver_vibrate = test_device
        .endpoint_receiver(&Endpoint::TxVibrate)
        .expect("Test, assuming infallible");
      let command_receiver_mode = test_device
        .endpoint_receiver(&Endpoint::TxMode)
        .expect("Test, assuming infallible");

      assert!(check_test_recv_empty(&command_receiver_mode));
      assert!(check_test_recv_empty(&command_receiver_vibrate));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_vibrate,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x80, 0x00, 0x0],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(1, 1.0)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_vibrate,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x80, 0xff, 0x0],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(2, 1.0)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_vibrate,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x80, 0xff, 0x02],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));

      // Since we only created one subcommand, we should only receive one command.
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver_mode));
      assert!(check_test_recv_empty(&command_receiver_vibrate));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver_vibrate,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxVibrate,
          vec![0x0, 0x0, 0x0],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_vibrate));
      check_test_recv_value(
        &command_receiver_mode,
        HardwareCommand::Write(HardwareWriteCmd::new(
          Endpoint::TxMode,
          vec![0x03, 0xff],
          false,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver_mode));
    });
  }
}
*/
