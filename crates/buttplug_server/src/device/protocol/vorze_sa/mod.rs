// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod dual_rotator;
mod piston;
mod single_rotator;
mod vibrator;

use crate::device::{
  hardware::Hardware,
  protocol::{
    generic_protocol_initializer_setup,
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
  },
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::{
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};
use std::sync::Arc;

generic_protocol_initializer_setup!(VorzeSA, "vorze-sa");

#[derive(Default)]
pub struct VorzeSAInitializer {}

#[async_trait]
impl ProtocolInitializer for VorzeSAInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    def: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    if let Some(variant) = def.protocol_variant() {
      let hwname = hardware.name().to_ascii_lowercase();
      match variant.as_str() {
        "vorze-sa-single-rotator" => {
          if hwname.contains("cycsa") {
            Ok(Arc::new(single_rotator::VorzeSASingleRotator::new(
              VorzeDevice::Cyclone,
            )))
          } else if hwname.contains("ufo") {
            Ok(Arc::new(single_rotator::VorzeSASingleRotator::new(
              VorzeDevice::Ufo,
            )))
          } else {
            Err(ButtplugDeviceError::ProtocolNotImplemented(format!(
              "No protocol implementation for Vorze Device {}",
              hardware.name()
            )))
          }
        }
        "vorze-sa-dual-rotator" => Ok(Arc::new(dual_rotator::VorzeSADualRotator::default())),
        "vorze-sa-vibrator" => {
          if hwname.contains("bach") {
            Ok(Arc::new(vibrator::VorzeSAVibrator::new(VorzeDevice::Bach)))
          } else if hwname.contains("rocket") {
            Ok(Arc::new(vibrator::VorzeSAVibrator::new(
              VorzeDevice::Rocket,
            )))
          } else {
            Err(ButtplugDeviceError::ProtocolNotImplemented(format!(
              "No protocol implementation for Vorze Device {}",
              hardware.name()
            )))
          }
        }
        "vorze-sa-piston" => Ok(Arc::new(piston::VorzeSAPiston::default())),
        _ => Err(ButtplugDeviceError::ProtocolNotImplemented(format!(
          "No protocol implementation for Vorze Device {}",
          hardware.name()
        ))),
      }
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
