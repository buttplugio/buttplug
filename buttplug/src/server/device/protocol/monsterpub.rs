// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::device::{
    configuration::{ProtocolAttributesType, ProtocolDeviceAttributes},
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
  pub struct MonsterPubIdentifierFactory {}

  impl ProtocolIdentifierFactory for MonsterPubIdentifierFactory {
    fn identifier(&self) -> &str {
      "monsterpub"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::MonsterPubIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct MonsterPubIdentifier {}

#[async_trait]
impl ProtocolIdentifier for MonsterPubIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<(ServerDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let read_resp = hardware
      .read_value(&HardwareReadCmd::new(Endpoint::RxBLEModel, 32, 500))
      .await;
    let ident = match read_resp {
      Ok(data) => std::str::from_utf8(&data.data())
        .map_err(|_| {
          ButtplugDeviceError::ProtocolSpecificError(
            "monsterpub".to_owned(),
            "MonsterPub device name is non-UTF8 string.".to_owned(),
          )
        })?
        .replace("\0", "")
        .to_owned(),
      Err(_) => "Unknown".to_string(),
    };
    return Ok((
      ServerDeviceIdentifier::new(
        hardware.address(),
        "monsterpub",
        &ProtocolAttributesType::Identifier(ident),
      ),
      Box::new(MonsterPubInitializer::default()),
    ));
  }
}

#[derive(Default)]
pub struct MonsterPubInitializer {}

#[async_trait]
impl ProtocolInitializer for MonsterPubInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    if hardware.endpoints().contains(&Endpoint::Rx) {
      let value = hardware
        .read_value(&HardwareReadCmd::new(Endpoint::Rx, 16, 200))
        .await?;
      let keys = [
        [
          0x32u8, 0x49, 0x50, 0x4f, 0x32, 0x49, 0x50, 0x4f, 0x32, 0x49, 0x50, 0x4f, 0x32, 0x49,
          0x50,
        ],
        [
          0x4c, 0x53, 0x42, 0x42, 0x4c, 0x53, 0x42, 0x42, 0x4c, 0x53, 0x42, 0x42, 0x4c, 0x53, 0x42,
        ],
        [
          0x53, 0x49, 0x53, 0x36, 0x53, 0x49, 0x53, 0x36, 0x53, 0x49, 0x53, 0x36, 0x53, 0x49, 0x53,
        ],
        [
          0x54, 0x41, 0x4c, 0x4b, 0x54, 0x41, 0x4c, 0x4b, 0x54, 0x41, 0x4c, 0x4b, 0x54, 0x41, 0x4c,
        ],
      ];

      let auth = value.data()[1..16]
        .iter()
        .zip(keys[value.data()[0] as usize].iter())
        .map(|(&x1, &x2)| x1 ^ x2)
        .collect();

      trace!(
        "Got {:?} XOR with key {} to get {:?}",
        value.data(),
        value.data()[0],
        auth
      );

      hardware
        .write_value(&HardwareWriteCmd::new(Endpoint::Rx, auth, true))
        .await?;
    }
    Ok(Arc::new(MonsterPub::new(
      if hardware.endpoints().contains(&Endpoint::TxVibrate) {
        Endpoint::TxVibrate
      } else {
        Endpoint::Tx
      },
    )))
  }
}

pub struct MonsterPub {
  tx: Endpoint,
}

impl MonsterPub {
  pub fn new(tx: Endpoint) -> Self {
    Self { tx }
  }
}

impl ProtocolHandler for MonsterPub {
  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut data = vec![];
    let mut stop = true;
    for (_, cmd) in cmds.iter().enumerate() {
      if let Some((_, speed)) = cmd {
        data.push(*speed as u8);
        if *speed != 0 {
          stop = false;
        }
      }
    }
    let tx = if self.tx == Endpoint::Tx && stop {
      Endpoint::TxMode
    } else {
      self.tx
    };
    Ok(vec![HardwareWriteCmd::new(
      tx,
      data,
      tx == Endpoint::TxMode,
    )
    .into()])
  }
}
