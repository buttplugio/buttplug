// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::message::OutputType;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use uuid::{uuid, Uuid};

const VIBRATISSIMO_PROTOCOL_UUID: Uuid = uuid!("66ef7aa4-1e6a-4067-9066-dcb53c7647f2");

pub mod setup {
  use crate::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct VibratissimoIdentifierFactory {}

  impl ProtocolIdentifierFactory for VibratissimoIdentifierFactory {
    fn identifier(&self) -> &str {
      "vibratissimo"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::VibratissimoIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct VibratissimoIdentifier {}

#[async_trait]
impl ProtocolIdentifier for VibratissimoIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let result = hardware
      .read_value(&HardwareReadCmd::new(
        VIBRATISSIMO_PROTOCOL_UUID,
        Endpoint::RxBLEModel,
        128,
        500,
      ))
      .await?;
    let ident =
      String::from_utf8(result.data().to_vec()).unwrap_or_else(|_| hardware.name().to_owned());
    Ok((
      UserDeviceIdentifier::new(hardware.address(), "vibratissimo", &Some(ident)),
      Box::new(VibratissimoInitializer::default()),
    ))
  }
}

#[derive(Default)]
pub struct VibratissimoInitializer {}

#[async_trait]
impl ProtocolInitializer for VibratissimoInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    def: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let num_vibrators: u8 = def
      .features()
      .iter()
      .filter(|x| {
        x.output()
          .as_ref()
          .map_or(false, |x| x.contains_key(&OutputType::Vibrate))
      })
      .count() as u8;
    Ok(Arc::new(Vibratissimo::new(num_vibrators as u8)))
  }
}

pub struct Vibratissimo {
  speeds: Vec<AtomicU8>,
}

impl Vibratissimo {
  fn new(num_vibrators: u8) -> Self {
    let speeds: Vec<AtomicU8> = std::iter::repeat_with(|| AtomicU8::default())
      .take(num_vibrators as usize)
      .collect();
    Self { speeds }
  }
}

impl ProtocolHandler for Vibratissimo {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    let mut data = vec![];
    for cmd in &self.speeds {
      data.push(cmd.load(std::sync::atomic::Ordering::Relaxed));
    }
    if data.len() == 1 {
      data.push(0x00);
    }

    // Put the device in write mode
    Ok(vec![
      HardwareWriteCmd::new(&[feature_id], Endpoint::TxMode, vec![0x03, 0xff], false).into(),
      HardwareWriteCmd::new(&[feature_id], Endpoint::TxVibrate, data, false).into(),
    ])
  }
}
