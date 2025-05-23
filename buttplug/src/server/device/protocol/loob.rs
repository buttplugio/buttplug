// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::Endpoint,
  },
  server::{device::{
    configuration::{ProtocolCommunicationSpecifier, UserDeviceDefinition, UserDeviceIdentifier},
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
  }, message::checked_value_with_parameter_cmd::CheckedValueWithParameterCmdV4},
};
use async_trait::async_trait;
use uuid::{uuid, Uuid};
use std::cmp::{max, min};
use std::sync::Arc;

const LOOB_PROTOCOL_UUID: Uuid = uuid!("b3a02457-3bda-4c5b-8363-aead6eda74ae");
generic_protocol_initializer_setup!(Loob, "loob");

#[derive(Default)]
pub struct LoobInitializer {}

#[async_trait]
impl ProtocolInitializer for LoobInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let msg = HardwareWriteCmd::new(LOOB_PROTOCOL_UUID, Endpoint::Tx, vec![0x00, 0x01, 0x01, 0xf4], true);
    hardware.write_value(&msg).await?;
    Ok(Arc::new(Loob::default()))
  }
}

#[derive(Default)]
pub struct Loob {}

impl ProtocolHandler for Loob {
  fn handle_position_with_duration_cmd(
    &self,
    message: &CheckedValueWithParameterCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let pos: u16 = max(min(message.value() as u16, 1000), 1);
    let time: u16 = max(message.parameter() as u16, 1);
    let mut data = pos.to_be_bytes().to_vec();
    for b in time.to_be_bytes() {
      data.push(b);
    }
    Ok(vec![HardwareWriteCmd::new(message.feature_id(), Endpoint::Tx, data, false).into()])
  }
}
