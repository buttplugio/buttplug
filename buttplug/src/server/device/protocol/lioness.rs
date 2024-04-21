// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    configuration::{ProtocolDeviceAttributes, UserDeviceIdentifier},
    hardware::{Hardware, HardwareCommand, HardwareSubscribeCmd, HardwareWriteCmd},
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

generic_protocol_initializer_setup!(Lioness, "lioness");

#[derive(Default)]
pub struct LionessInitializer {}

#[async_trait]
impl ProtocolInitializer for LionessInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    hardware
      .subscribe(&HardwareSubscribeCmd::new(Endpoint::Rx))
      .await?;

    let res = hardware
      .write_value(&HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![0x01, 0xAA, 0xAA, 0xBB, 0xCC, 0x10],
        true,
      ))
      .await;

    if res.is_err() {
      return Err(ButtplugDeviceError::DeviceCommunicationError(
        "Lioness may need pairing with OS. Use PIN 6496 or 006496 when pairing.".to_string(),
      ));
    }
    Ok(Arc::new(Lioness::default()))
  }
}

#[derive(Default)]
pub struct Lioness {}

impl ProtocolHandler for Lioness {
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
      vec![0x02, 0xAA, 0xBB, 0xCC, 0xCC, scalar as u8],
      false,
    )
    .into()])
  }
}
