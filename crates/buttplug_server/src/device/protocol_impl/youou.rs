// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
};
use async_trait::async_trait;
use std::sync::{
  atomic::{AtomicU8, Ordering},
  Arc,
};
use uuid::Uuid;

pub mod setup {
  use crate::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct YououIdentifierFactory {}

  impl ProtocolIdentifierFactory for YououIdentifierFactory {
    fn identifier(&self) -> &str {
      "youou"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::YououIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct YououIdentifier {}

#[async_trait]
impl ProtocolIdentifier for YououIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    Ok((
      UserDeviceIdentifier::new(hardware.address(), "Youou", &Some("VX001_".to_owned())),
      Box::new(YououInitializer::default()),
    ))
  }
}

#[derive(Default)]
pub struct YououInitializer {}

#[async_trait]
impl ProtocolInitializer for YououInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(Youou::default()))
  }
}

#[derive(Default)]
pub struct Youou {
  packet_id: AtomicU8,
}

impl ProtocolHandler for Youou {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // Byte 2 seems to be a monotonically increasing packet id of some kind
    //
    // Speed seems to be 0-247 or so.
    //
    // Anything above that sets a pattern which isn't what we want here.
    let state = u8::from(speed > 0);

    // Scope the packet id set so we can unlock ASAP.
    let mut data = vec![
      0xaa,
      0x55,
      self.packet_id.load(Ordering::Relaxed),
      0x02,
      0x03,
      0x01,
      speed as u8,
      state,
    ];
    self.packet_id.store(
      self.packet_id.load(Ordering::Relaxed).wrapping_add(1),
      Ordering::Relaxed,
    );
    let mut crc: u8 = 0;

    // Simple XOR of everything up to the 9th byte for CRC.
    for b in data.clone() {
      crc ^= b;
    }

    let mut data2 = vec![crc, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    data.append(&mut data2);

    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      data,
      false,
    )
    .into()])
  }
}
