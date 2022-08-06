// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, messages::Endpoint},
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

generic_protocol_initializer_setup!(Ankni, "ankni");

#[derive(Default)]
pub struct AnkniInitializer {}

#[async_trait]
impl ProtocolInitializer for AnkniInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
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
    Ok(Arc::new(Ankni::default()))
  }
}

#[derive(Default)]
pub struct Ankni {}

impl ProtocolHandler for Ankni {
  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
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
    )
    .into()])
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use super::Ankni;
  use crate::{
    core::messages::{Endpoint, ActuatorType},
    server::device::{
      protocol::ProtocolHandler,
      hardware::{HardwareCommand, HardwareWriteCmd},
    },
  };

  #[test]
  pub fn test_ankni_protocol() {
    let handler = Ankni {};
    assert_eq!(
      handler.handle_scalar_cmd(&vec![Some((ActuatorType::Vibrate, 0x02))]),
      Ok(vec![HardwareCommand::Write(HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![
            0x03, 0x12, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
        true
      ))])
    );
    assert_eq!(
      handler.handle_scalar_cmd(&vec![
        Some((ActuatorType::Vibrate, 0x00)),
      ]),
      Ok(vec![
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0x03, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
          0x00, 0x00, 0x00, 0x00, 0x00, 0x00], true)),
      ])
    );
  }
}

