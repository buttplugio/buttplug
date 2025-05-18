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
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
  }, message::checked_value_cmd::CheckedValueCmdV4},
};
use async_trait::async_trait;
use std::sync::Arc;

generic_protocol_initializer_setup!(Nobra, "nobra");

#[derive(Default)]
pub struct NobraInitializer {}

#[async_trait]
impl ProtocolInitializer for NobraInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    hardware
      .write_value(&HardwareWriteCmd::new(Endpoint::Tx, vec![0x70], false))
      .await?;
    Ok(Arc::new(Nobra::default()))
  }
}

#[derive(Default)]
pub struct Nobra {}

impl ProtocolHandler for Nobra {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_value_vibrate_cmd(
    &self,
    cmd: &CheckedValueCmdV4
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let output_speed = if cmd.value() == 0 { 0x70 } else { 0x60 + cmd.value() };
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![output_speed as u8],
      false,
    )
    .into()])
  }
}
