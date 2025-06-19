// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod vibrator;
mod cyclone;
mod piston;
mod ufo;

use crate::{
  core::errors::ButtplugDeviceError,
  server::device::{
    configuration::{ProtocolCommunicationSpecifier, UserDeviceDefinition, UserDeviceIdentifier},
    hardware::Hardware,
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

generic_protocol_initializer_setup!(VorzeSA, "vorze-sa");

#[derive(Default)]
pub struct VorzeSAInitializer {}

#[async_trait]
impl ProtocolInitializer for VorzeSAInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let hwname = hardware.name().to_ascii_lowercase();
    if hwname.contains("cycsa") {
      Ok(Arc::new(cyclone::VorzeSACyclone::default()))
    } else if hwname.contains("ufo-tw") {
      Ok(Arc::new(ufo::VorzeSAUfo::new(VorzeDevice::UfoTw)))
    } else if hwname.contains("ufo") {
      Ok(Arc::new(ufo::VorzeSAUfo::new(VorzeDevice::Ufo)))
    } else if hwname.contains("bach") {
      Ok(Arc::new(vibrator::VorzeSAVibrator::new(VorzeDevice::Bach)))
    } else if hwname.contains("rocket") {
      Ok(Arc::new(vibrator::VorzeSAVibrator::new(VorzeDevice::Rocket)))
    } else if hwname.contains("piston") {
      Ok(Arc::new(piston::VorzeSAPiston::default()))
    } else {
      Err(ButtplugDeviceError::ProtocolNotImplemented(format!(
        "No protocol implementation for Vorze Device {}",
        hardware.name()
      )))
    }
  }
}

#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum VorzeDevice {
  Bach = 6,
  Piston = 3,
  Cyclone = 1,
  Rocket = 7,
  Ufo = 2,
  UfoTw = 5,
}

#[repr(u8)]
enum VorzeActions {
  Rotate = 1,
  Vibrate = 3,
}
