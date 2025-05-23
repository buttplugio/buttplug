// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::hardware::Hardware;
use crate::server::device::protocol::ProtocolInitializer;
use crate::server::message::checked_value_cmd::CheckedValueCmdV4;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    configuration::{ProtocolCommunicationSpecifier, UserDeviceDefinition, UserDeviceIdentifier},
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_initializer_setup, ProtocolHandler, ProtocolIdentifier},
  },
};
use async_trait::async_trait;
use uuid::{uuid, Uuid};
use std::sync::Arc;

const METAXSIRE_V2_PROTOCOL_ID: Uuid = uuid!("28b934b4-ca45-4e14-85e7-4c1524b2b4c1");
generic_protocol_initializer_setup!(MetaXSireV2, "metaxsire-v2");

#[derive(Default)]
pub struct MetaXSireV2Initializer {}

#[async_trait]
impl ProtocolInitializer for MetaXSireV2Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    hardware
      .write_value(&HardwareWriteCmd::new(METAXSIRE_V2_PROTOCOL_ID, Endpoint::Tx, vec![0xaa, 0x04], true))
      .await?;
    Ok(Arc::new(MetaXSireV2::default()))
  }
}

#[derive(Default)]
pub struct MetaXSireV2 {}

impl ProtocolHandler for MetaXSireV2 {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_value_vibrate_cmd(
    &self,
    commands: &CheckedValueCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
        commands.feature_id(),
        Endpoint::Tx,
        vec![0xaa, 0x03, 0x01, (commands.feature_index() + 1) as u8, 0x64, commands.value() as u8],
        true,
    ).into()])
  }
}
