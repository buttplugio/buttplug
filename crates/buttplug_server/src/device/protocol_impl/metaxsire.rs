// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use uuid::{uuid, Uuid};

use buttplug_core::{
  errors::ButtplugDeviceError,
  message::OutputType,
};
use buttplug_server_device_config::{DeviceDefinition, Endpoint, ProtocolCommunicationSpecifier, UserDeviceIdentifier};

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{
    generic_protocol_initializer_setup,
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
  },
};

generic_protocol_initializer_setup!(MetaXSire, "metaxsire");

#[derive(Default)]
pub struct MetaXSireInitializer {}

#[async_trait]
impl ProtocolInitializer for MetaXSireInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    def: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let mut commands = vec![];
    def.features().iter().for_each(|x| {
      if x.output().is_some() {
        commands.push((x.feature_type().try_into().unwrap(), AtomicU8::default()))
      }
    });
    Ok(Arc::new(MetaXSire::new(commands)))
  }
}

const METAXSIRE_PROTOCOL_UUID: Uuid = uuid!("6485a762-2ea7-48c1-a4ba-ab724e618348");

#[derive(Default)]
pub struct MetaXSire {
  commands: Vec<(OutputType, AtomicU8)>,
}

impl MetaXSire {
  fn new(commands: Vec<(OutputType, AtomicU8)>) -> Self {
    Self { commands }
  }

  fn form_command(
    &self,
    feature_index: u32,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.commands[feature_index as usize]
      .1
      .store(speed as u8, Ordering::Relaxed);
    let mut data: Vec<u8> = vec![0x23, 0x07];
    data.push((self.commands.len() * 3) as u8);

    for (i, (output_type, speed)) in self.commands.iter().enumerate() {
      // motor number
      data.push(0x80 | ((i + 1) as u8));
      // motor type: 03=vibe 04=pump 06=rotate
      data.push(if *output_type == OutputType::Rotate {
        0x06
      } else if *output_type == OutputType::Constrict || *output_type == OutputType::Oscillate {
        0x04
      } else {
        // Vibrate
        0x03
      });
      data.push(speed.load(Ordering::Relaxed));
    }

    let mut crc: u8 = 0;
    for b in data.clone() {
      crc ^= b;
    }
    data.push(crc);

    Ok(vec![HardwareWriteCmd::new(
      &[METAXSIRE_PROTOCOL_UUID],
      Endpoint::Tx,
      data,
      false,
    )
    .into()])
  }
}

impl ProtocolHandler for MetaXSire {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_command(feature_index, speed)
  }

  fn handle_output_oscillate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_command(feature_index, speed)
  }

  fn handle_output_rotate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_command(feature_index, speed)
  }

  fn handle_output_constrict_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_command(feature_index, level)
  }
}
