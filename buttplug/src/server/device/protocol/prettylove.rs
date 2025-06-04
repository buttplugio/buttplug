// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::{device::{
    configuration::{ProtocolCommunicationSpecifier, UserDeviceDefinition, UserDeviceIdentifier},
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
  }, message::checked_actuator_cmd::CheckedActuatorCmdV4},
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
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    Ok((
      UserDeviceIdentifier::new(
        hardware.address(),
        "prettylove",
        &Some("Aogu BLE".to_owned()),
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
    _: &UserDeviceDefinition,
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

    fn handle_value_vibrate_cmd(
    &self,
    cmd: &CheckedActuatorCmdV4
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      cmd.feature_id(),
      Endpoint::Tx,
      vec![0x00u8, cmd.value() as u8],
      true,
    )
    .into()])
  }
}
