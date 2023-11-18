// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::configuration::ProtocolDeviceAttributes;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
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
  pub struct PrettyLoveIdentifierFactory {}

  impl ProtocolIdentifierFactory for PrettyLoveIdentifierFactory {
    fn identifier(&self) -> &str {
      "prettylove"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::PrettyLoveIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct PrettyLoveIdentifier {}

#[async_trait]
impl ProtocolIdentifier for PrettyLoveIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<(ServerDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    Ok((
      ServerDeviceIdentifier::new(
        hardware.address(),
        "prettylove",
        &ProtocolAttributesType::Identifier("Aogu BLE".to_owned()),
      ),
      Box::new(PrettyLoveInitializer::default()),
    ))
  }
}

#[derive(Default)]
pub struct PrettyLoveInitializer {}

#[async_trait]
impl ProtocolInitializer for PrettyLoveInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(PrettyLove::default()))
  }
}

#[derive(Default)]
pub struct PrettyLove {}

impl ProtocolHandler for PrettyLove {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0x00u8, scalar as u8],
      true,
    )
    .into()])
  }
}
