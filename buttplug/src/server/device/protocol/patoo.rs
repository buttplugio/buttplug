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
    protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
    ServerDeviceIdentifier,
  },
};
use async_trait::async_trait;
use std::sync::Arc;

pub mod setup {
  use crate::server::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct PatooIdentifierFactory {}

  impl ProtocolIdentifierFactory for PatooIdentifierFactory {
    fn identifier(&self) -> &str {
      "patoo"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::PatooIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct PatooIdentifier {}

#[async_trait]
impl ProtocolIdentifier for PatooIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<(ServerDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    // Patoo Love devices have wildcarded names of ([A-Z]+)\d*
    // Force the identifier lookup to the non-numeric portion
    let c: Vec<char> = hardware.name().chars().collect();
    let mut i = 0;
    while i < c.len() && !c[i].is_ascii_digit() {
      i += 1;
    }
    let name: String = c[0..i].iter().collect();
    Ok((
      ServerDeviceIdentifier::new(
        hardware.address(),
        "Patoo",
        &ProtocolAttributesType::Identifier(name),
      ),
      Box::new(PatooInitializer::default()),
    ))
  }
}

#[derive(Default)]
pub struct PatooInitializer {}

#[async_trait]
impl ProtocolInitializer for PatooInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(Patoo::default()))
  }
}

#[derive(Default)]
pub struct Patoo {}

impl ProtocolHandler for Patoo {
  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut msg_vec = vec![];
    // Default to vibes
    let mut mode: u8 = 4u8;

    // Use vibe 1 as speed
    let mut speed = cmds[0].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8;
    if speed == 0 {
      mode = 0;

      // If we have a second vibe and it's not also 0, use that
      if cmds.len() > 1 {
        speed = cmds[1].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8;
        if speed != 0 {
          mode |= 0x80;
        }
      }
    } else if cmds.len() > 1 && cmds[1].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8 != 0 {
      // Enable second vibe if it's not at 0
      mode |= 0x80;
    }

    msg_vec.push(HardwareWriteCmd::new(Endpoint::Tx, vec![speed], true).into());
    msg_vec.push(HardwareWriteCmd::new(Endpoint::TxMode, vec![mode], true).into());

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
  pub fn test_patoo_protocol_devil() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("PBT821")
        .await
        .expect("Test, assuming infallible");
      let command_receiver_tx = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      let command_receiver_txmode = test_device
        .endpoint_receiver(&Endpoint::TxMode)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      // We just vibe 1 so expect 2 writes (mode 0x04)
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![50], true)),
      );
      check_test_recv_value(
        &command_receiver_txmode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x04], true)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      // no-op
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));

      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.1),
              VibrateSubcommand::new(1, 0.5),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      // setting second vibe whilst changing vibe 1, 2 writes (mode 1)
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![10], true)),
      );
      check_test_recv_value(
        &command_receiver_txmode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x84], true)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));

      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.1),
              VibrateSubcommand::new(1, 0.9),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      // only vibe 1 changed, 2 writes, same data
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![10], true)),
      );
      check_test_recv_value(
        &command_receiver_txmode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x84], true)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));

      device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.0),
              VibrateSubcommand::new(1, 0.9),
            ],
          )
          .into(),
        )
        .await
        .expect("Test, assuming infallible");
      // turn off vibe 1, 2 writes (mode 0x80)
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![90], true)),
      );
      check_test_recv_value(
        &command_receiver_txmode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x80], true)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      // stop on both, 2 writes (mode 0)
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0], true)),
      );
      check_test_recv_value(
        &command_receiver_txmode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0], true)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));
    });
  }

  #[test]
  pub fn test_patoo_protocol_carrot() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("PTVEA2601")
        .await
        .expect("Test, assuming infallible");

      let command_receiver_tx = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      let command_receiver_txmode = test_device
        .endpoint_receiver(&Endpoint::TxMode)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      // We just vibe 1 so expect 2 writes (mode 0x04)
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![50], true)),
      );
      check_test_recv_value(
        &command_receiver_txmode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0x04], true)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));

      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      // no-op
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));

      assert!(device
        .parse_message(
          VibrateCmd::new(
            0,
            vec![
              VibrateSubcommand::new(0, 0.1),
              VibrateSubcommand::new(1, 0.5),
            ],
          )
          .into(),
        )
        .await
        .is_err());
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      // stop on both, 2 writes (mode 0)
      check_test_recv_value(
        &command_receiver_tx,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0], true)),
      );
      check_test_recv_value(
        &command_receiver_txmode,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::TxMode, vec![0], true)),
      );
      assert!(check_test_recv_empty(&command_receiver_tx));
      assert!(check_test_recv_empty(&command_receiver_txmode));
    });
  }
}
 */
