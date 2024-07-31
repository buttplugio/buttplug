// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint, FeatureType},
  },
  server::device::{
    configuration::{ProtocolCommunicationSpecifier, UserDeviceDefinition, UserDeviceIdentifier},
    hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
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

generic_protocol_initializer_setup!(SenseeV2, "sensee-v2");

#[derive(Default)]
pub struct SenseeV2Initializer {}

#[async_trait]
impl ProtocolInitializer for SenseeV2Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    device_definition: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let res = hardware
      .read_value(&HardwareReadCmd::new(Endpoint::Tx, 128, 500))
      .await?;
    info!("Sensee model data: {:X?}", res.data());
    let mut protocol = SenseeV2::default();
    protocol.device_type = if res.data().len() >= 6 {
      res.data()[6]
    } else {
      0x66
    };

    protocol.vibe_count = device_definition
      .features()
      .iter()
      .filter(|x| [FeatureType::Vibrate].contains(x.feature_type()))
      .count();
    protocol.thrust_count = device_definition
      .features()
      .iter()
      .filter(|x| [FeatureType::Oscillate].contains(x.feature_type()))
      .count();
    protocol.suck_count = device_definition
      .features()
      .iter()
      .filter(|x| [FeatureType::Constrict].contains(x.feature_type()))
      .count();

    Ok(Arc::new(protocol))
  }
}

#[derive(Default)]
pub struct SenseeV2 {
  device_type: u8,
  vibe_count: usize,
  thrust_count: usize,
  suck_count: usize,
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

impl ProtocolHandler for SenseeV2 {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }
  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn handle_scalar_cmd(
    &self,
    commands: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let vibes: Vec<(ActuatorType, u32)> = commands
      .iter()
      .map(|x| x.expect("Expecting all commands"))
      .filter(|x| x.0 == ActuatorType::Vibrate)
      .collect();
    let thrusts: Vec<(ActuatorType, u32)> = commands
      .iter()
      .map(|x| x.expect("Expecting all commands"))
      .filter(|x| x.0 == ActuatorType::Oscillate)
      .collect();
    let sucks: Vec<(ActuatorType, u32)> = commands
      .iter()
      .map(|x| x.expect("Expecting all commands"))
      .filter(|x| x.0 == ActuatorType::Constrict)
      .collect();

    let mut data = vec![];
    data.push(
      if self.vibe_count != 0 { 1 } else { 0 }
        + if self.thrust_count != 0 { 1 } else { 0 }
        + if self.suck_count != 0 { 1 } else { 0 } as u8,
    );
    if self.vibe_count != 0 {
      data.push(0);
      data.push(self.vibe_count as u8);
      for i in 0..self.vibe_count {
        data.push((i + 1) as u8);
        data.push(vibes[i].1 as u8);
      }
    }
    if self.thrust_count != 0 {
      data.push(1);
      data.push(self.thrust_count as u8);
      for i in 0..self.thrust_count {
        data.push((i + 1) as u8);
        data.push(thrusts[i].1 as u8);
      }
    }
    if self.suck_count != 0 {
      data.push(2);
      data.push(self.suck_count as u8);
      for i in 0..self.suck_count {
        data.push((i + 1) as u8);
        data.push(sucks[i].1 as u8);
      }
    }

    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      make_cmd(self.device_type, 0xf1, data),
      false,
    )
    .into()])
  }
}
