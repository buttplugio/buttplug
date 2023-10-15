// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::ActuatorType::Vibrate;
use crate::server::device::configuration::ProtocolDeviceAttributes;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    configuration::ProtocolAttributesType,
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
  pub struct HismithMiniIdentifierFactory {}

  impl ProtocolIdentifierFactory for HismithMiniIdentifierFactory {
    fn identifier(&self) -> &str {
      "hismith-mini"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::HismithMiniIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct HismithMiniIdentifier {}

#[async_trait]
impl ProtocolIdentifier for HismithMiniIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<(ServerDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let result = hardware
      .read_value(&HardwareReadCmd::new(Endpoint::RxBLEModel, 128, 500))
      .await?;

    let identifier = result
      .data()
      .iter()
      .map(|b| format!("{:02x}", b))
      .collect::<String>();
    info!("Hismith Device Identifier: {}", identifier);

    Ok((
      ServerDeviceIdentifier::new(
        hardware.address(),
        "hismith-mini",
        &ProtocolAttributesType::Identifier(identifier),
      ),
      Box::new(HismithMiniInitializer::default()),
    ))
  }
}

#[derive(Default)]
pub struct HismithMiniInitializer {}

#[async_trait]
impl ProtocolInitializer for HismithMiniInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    attrs: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let mut dual_vibes = false;
    if let Some(scalar) = attrs.message_attributes().scalar_cmd() {
      dual_vibes = scalar
        .iter()
        .filter(|s| s.actuator_type() == &Vibrate)
        .count()
        >= 2;
    }
    Ok(Arc::new(HismithMini {
      dual_vibe: dual_vibes,
    }))
  }
}

#[derive(Default)]
pub struct HismithMini {
  dual_vibe: bool,
}

impl ProtocolHandler for HismithMini {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_oscillate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let idx: u8 = 0x03;
    let speed: u8 = scalar as u8;

    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0xCC, idx, speed, speed + idx],
      false,
    )
    .into()])
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let idx: u8 = if !self.dual_vibe || index == 1 {
      0x05
    } else {
      0x03
    };
    let speed: u8 = scalar as u8;

    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0xCC, idx, speed, speed + idx],
      false,
    )
    .into()])
  }

  fn handle_scalar_constrict_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let idx: u8 = 0x03;
    let speed: u8 = scalar as u8;

    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0xCC, idx, speed, speed + idx],
      false,
    )
    .into()])
  }
}
