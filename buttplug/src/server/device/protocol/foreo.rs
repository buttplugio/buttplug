// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::configuration::ProtocolCommunicationSpecifier;
use crate::server::message::checked_value_cmd::CheckedValueCmdV4;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    configuration::{UserDeviceDefinition, UserDeviceIdentifier},
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
  },
};
use async_trait::async_trait;
use std::sync::Arc;

generic_protocol_initializer_setup!(Foreo, "foreo");

#[derive(Default)]
pub struct ForeoInitializer {}

#[async_trait]
impl ProtocolInitializer for ForeoInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let lname = hardware.name().to_lowercase();
    let mut ph = Foreo::default();
    ph.mode = 0;

    if lname.contains("smart") && lname.contains("2") {
      ph.mode = 3;
    } else if lname.contains("fofo") || lname.contains("ufo") {
      ph.mode = 1;
    }

    Ok(Arc::new(ph))
  }
}

#[derive(Default)]
pub struct Foreo {
  mode: u8,
}

impl ProtocolHandler for Foreo {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_value_vibrate_cmd(
    &self,
    cmd: &CheckedValueCmdV4
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      cmd.feature_uuid(),
      Endpoint::Tx,
      vec![0x01, self.mode, cmd.value() as u8],
      true,
    )
    .into()])
  }
}
