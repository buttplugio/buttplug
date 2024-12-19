// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::configuration::ProtocolCommunicationSpecifier;
use crate::server::device::protocol::hismith_mini::HismithMiniInitializer;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    configuration::{UserDeviceDefinition, UserDeviceIdentifier},
    hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
    protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
  },
};
use async_trait::async_trait;
use std::sync::Arc;

pub mod setup {
  use crate::server::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct HismithIdentifierFactory {}

  impl ProtocolIdentifierFactory for HismithIdentifierFactory {
    fn identifier(&self) -> &str {
      "hismith"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::HismithIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct HismithIdentifier {}

const LEGACY_HISMITHS: [&'static str; 6] = ["1001", "1002", "1003", "3001", "2001", "1006"];

#[async_trait]
impl ProtocolIdentifier for HismithIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let result = hardware
      .read_value(&HardwareReadCmd::new(Endpoint::RxBLEModel, 128, 500))
      .await?;

    let identifier = result
      .data()
      .iter()
      .map(|b| format!("{:02x}", b))
      .collect::<String>();
    info!("Hismith Device Identifier: {}", identifier);

    if !LEGACY_HISMITHS.contains(&identifier.as_str()) {
      info!("Not a legacy Hismith, using hismith-mini protocol");
      return Ok((
        UserDeviceIdentifier::new(hardware.address(), "hismith-mini", &Some(identifier)),
        Box::new(HismithMiniInitializer::default()),
      ));
    }

    Ok((
      UserDeviceIdentifier::new(hardware.address(), "hismith", &Some(identifier)),
      Box::new(HismithInitializer::default()),
    ))
  }
}

#[derive(Default)]
pub struct HismithInitializer {}

#[async_trait]
impl ProtocolInitializer for HismithInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(Hismith::default()))
  }
}

#[derive(Default)]
pub struct Hismith {}

impl ProtocolHandler for Hismith {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_oscillate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let idx: u8 = 0x04;
    let speed: u8 = scalar as u8;

    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0xAA, idx, speed, speed + idx],
      false,
    )
    .into()])
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // Wildolo has a vibe at index 0 using id 4
    // The thrusting stroker has a vibe at index 1 using id 6 (and the weird 0xf0 off)
    let idx: u8 = if index == 0 { 0x04 } else { 0x06 };
    let speed: u8 = if index != 0 && scalar == 0 {
      0xf0
    } else {
      scalar as u8
    };

    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0xAA, idx, speed, speed + idx],
      false,
    )
    .into()])
  }
}
