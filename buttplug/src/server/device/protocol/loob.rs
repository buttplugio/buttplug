// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{self, Endpoint},
  },
  server::device::{
    configuration::{ProtocolCommunicationSpecifier, UserDeviceDefinition, UserDeviceIdentifier},
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
use std::cmp::{max, min};
use std::sync::Arc;

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
    let msg = HardwareWriteCmd::new(Endpoint::Tx, vec![0x00, 0x01, 0x01, 0xf4], true);
    hardware.write_value(&msg).await?;
    Ok(Arc::new(Loob::default()))
  }
}

#[derive(Default)]
pub struct Loob {}

impl ProtocolHandler for Loob {
  fn handle_linear_cmd(
    &self,
    message: message::LinearCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if let Some(vec) = message.vectors().get(0) {
      let pos: u16 = max(min((vec.position() * 1000.0) as u16, 1000), 1);
      let time: u16 = max(vec.duration() as u16, 1);
      let mut data = pos.to_be_bytes().to_vec();
      for b in time.to_be_bytes() {
        data.push(b);
      }
      Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, false).into()])
    } else {
      Ok(vec![])
    }
  }
}
