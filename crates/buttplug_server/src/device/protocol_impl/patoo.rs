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
use uuid::{uuid, Uuid};

pub mod setup {
  use crate::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct PatooIdentifierFactory {}

  impl ProtocolIdentifierFactory for PatooIdentifierFactory {
    fn identifier(&self) -> &str {
      "patoo"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::PatooIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct PatooIdentifier {}

#[async_trait]
impl ProtocolIdentifier for PatooIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    // Patoo Love devices have wildcarded names of ([A-Z]+)\d*
    // Force the identifier lookup to the non-numeric portion
    let c: Vec<char> = hardware.name().chars().collect();
    let mut i = 0;
    while i < c.len() && !c[i].is_ascii_digit() {
      i += 1;
    }
    let name: String = c[0..i].iter().collect();
    Ok((
      UserDeviceIdentifier::new(hardware.address(), "Patoo", &Some(name)),
      Box::new(PatooInitializer::default()),
    ))
  }
}

#[derive(Default)]
pub struct PatooInitializer {}

#[async_trait]
impl ProtocolInitializer for PatooInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(Patoo::default()))
  }
}

const PATOO_TX_PROTOCOL_UUID: Uuid = uuid!("2366a70f-9a7c-4fea-8ba6-8b21a7d5d641");
const PATOO_TX_MODE_PROTOCOL_UUID: Uuid = uuid!("b17714be-fc66-4d9b-bf52-afb3b91212a4");

#[derive(Default)]
pub struct Patoo {
  speeds: [AtomicU8; 2],
}

impl ProtocolHandler for Patoo {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    let mut msg_vec = vec![];
    // Default to vibes
    let mut mode: u8 = 4u8;

    // Use vibe 1 as speed
    let mut speed = self.speeds[0].load(Ordering::Relaxed);
    if speed == 0 {
      mode = 0;

      let speed2 = self.speeds[1].load(Ordering::Relaxed);
      // If we have a second vibe and it's not also 0, use that
      if speed2 != 0 {
        speed = speed2;
        mode |= 0x80;
      }
    }

    msg_vec.push(
      HardwareWriteCmd::new(&[PATOO_TX_PROTOCOL_UUID], Endpoint::Tx, vec![speed], true).into(),
    );
    msg_vec.push(
      HardwareWriteCmd::new(
        &[PATOO_TX_MODE_PROTOCOL_UUID],
        Endpoint::TxMode,
        vec![mode],
        true,
      )
      .into(),
    );

    Ok(msg_vec)
  }
}
