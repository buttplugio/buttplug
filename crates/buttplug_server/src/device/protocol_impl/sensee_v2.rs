// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::{
  errors::ButtplugDeviceError,
  message::OutputType,
};
use buttplug_server_device_config::{
  DeviceDefinition, Endpoint, ProtocolCommunicationSpecifier, UserDeviceIdentifier
};

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
  protocol::{
    generic_protocol_initializer_setup,
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
  },
};
use async_trait::async_trait;
use std::{
  collections::HashMap,
  sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
  },
};
use uuid::{uuid, Uuid};

generic_protocol_initializer_setup!(SenseeV2, "sensee-v2");

const SENSEE_V2_PROTOCOL_UUID: Uuid = uuid!("6e68d015-6e83-484b-9dbc-de7684cf8c29");

#[derive(Default)]
pub struct SenseeV2Initializer {}

#[async_trait]
impl ProtocolInitializer for SenseeV2Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    device_definition: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let res = hardware
      .read_value(&HardwareReadCmd::new(
        SENSEE_V2_PROTOCOL_UUID,
        Endpoint::Tx,
        128,
        500,
      ))
      .await?;
    info!("Sensee model data: {:X?}", res.data());

    let device_type = if res.data().len() >= 6 {
      res.data()[6]
    } else {
      0x66
    };

    let feature_map = |output_type| {
      let mut map = HashMap::new();
      device_definition
        .features()
        .iter()
        .enumerate()
        .for_each(|(i, x)| {
          if let Some(output_map) = x.output() {
            if output_map.contains_key(&output_type) {
              map.insert(i as u32, AtomicU8::new(0));
            }
          }
        });
      map
    };

    let vibe_map = feature_map(OutputType::Vibrate);
    let thrust_map = feature_map(OutputType::Oscillate);
    let suck_map = feature_map(OutputType::Constrict);

    Ok(Arc::new(SenseeV2::new(
      device_type,
      vibe_map,
      thrust_map,
      suck_map,
    )))
  }
}

pub struct SenseeV2 {
  device_type: u8,
  vibe_map: HashMap<u32, AtomicU8>,
  thrust_map: HashMap<u32, AtomicU8>,
  suck_map: HashMap<u32, AtomicU8>,
}

fn make_cmd(dtype: u8, func: u8, cmd: Vec<u8>) -> Vec<u8> {
  let mut out = vec![0x55, 0xAA, 0xF0]; // fixed start code
  out.push(0x02); // version
  out.push(0x00); // package numer?
  out.push(0x04 + cmd.len() as u8); // Data length
  out.push(dtype); // Device type - always 0x66?
  out.push(func); // Function code
  out.extend(cmd);

  let cdc = vec![0, 0];
  // ToDo: CDC not yet used
  out.extend(cdc);

  out
}

impl SenseeV2 {
  fn new(
    device_type: u8,
    vibe_map: HashMap<u32, AtomicU8>,
    thrust_map: HashMap<u32, AtomicU8>,
    suck_map: HashMap<u32, AtomicU8>,
  ) -> Self {
    Self {
      device_type,
      vibe_map,
      thrust_map,
      suck_map,
    }
  }

  fn compile_command(&self) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut data = vec![];
    data.push(
      if self.vibe_map.len() != 0 { 1 } else { 0 }
        + if self.thrust_map.len() != 0 { 1 } else { 0 }
        + if self.suck_map.len() != 0 { 1 } else { 0 } as u8,
    );
    let mut data_add = |i, m: &HashMap<u32, AtomicU8>| {
      if m.len() > 0 {
        data.push(i);
        data.push(m.len() as u8);
        for (i, (_, v)) in m.iter().enumerate() {
          data.push((i + 1) as u8);
          data.push(v.load(Ordering::Relaxed));
        }
      }
    };
    data_add(0, &self.vibe_map);
    data_add(1, &self.thrust_map);
    data_add(2, &self.suck_map);

    Ok(vec![HardwareWriteCmd::new(
      &[SENSEE_V2_PROTOCOL_UUID],
      Endpoint::Tx,
      make_cmd(self.device_type, 0xf1, data),
      false,
    )
    .into()])
  }
}

impl ProtocolHandler for SenseeV2 {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self
      .vibe_map
      .get(&feature_index)
      .unwrap()
      .store(speed as u8, Ordering::Relaxed);
    self.compile_command()
  }

  fn handle_output_oscillate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self
      .thrust_map
      .get(&feature_index)
      .unwrap()
      .store(speed as u8, Ordering::Relaxed);
    self.compile_command()
  }

  fn handle_output_constrict_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self
      .suck_map
      .get(&feature_index)
      .unwrap()
      .store(level as u8, Ordering::Relaxed);
    self.compile_command()
  }
}
