// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer, ProtocolKeepaliveStrategy},
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};
use std::{
  sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
  },
  time::Duration,
};
use uuid::{uuid, Uuid};

const SATISFYER_PROTOCOL_UUID: Uuid = uuid!("79a0ed0d-f392-4c48-967e-f4467438c344");

pub mod setup {
  use crate::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct SatisfyerIdentifierFactory {}

  impl ProtocolIdentifierFactory for SatisfyerIdentifierFactory {
    fn identifier(&self) -> &str {
      "satisfyer"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::SatisfyerIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct SatisfyerIdentifier {}

#[async_trait]
impl ProtocolIdentifier for SatisfyerIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    specifier: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    if let ProtocolCommunicationSpecifier::BluetoothLE(s) = specifier {
      for md in s.manufacturer_data().iter() {
        if let Some(data) = md.data() {
          let device_identifier = format!(
            "{}",
            u32::from_be_bytes(data.to_vec().try_into().unwrap_or([0; 4]))
          );
          info!(
            "Satisfyer Device Identifier (from advertisement): {:?} {}",
            data, device_identifier
          );

          return Ok((
            UserDeviceIdentifier::new(hardware.address(), "satisfyer", &Some(device_identifier)),
            Box::new(SatisfyerInitializer::default()),
          ));
        }
      }
    }

    let result = hardware
      .read_value(&HardwareReadCmd::new(
        SATISFYER_PROTOCOL_UUID,
        Endpoint::RxBLEModel,
        128,
        500,
      ))
      .await?;
    let device_identifier = format!(
      "{}",
      u32::from_be_bytes(result.data().to_vec().try_into().unwrap_or([0; 4]))
    );
    info!(
      "Satisfyer Device Identifier (from RxBLEModel): {:?} {}",
      result.data(),
      device_identifier
    );
    return Ok((
      UserDeviceIdentifier::new(hardware.address(), "satisfyer", &Some(device_identifier)),
      Box::new(SatisfyerInitializer::default()),
    ));
  }
}

#[derive(Default)]
pub struct SatisfyerInitializer {}

#[async_trait]
impl ProtocolInitializer for SatisfyerInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    device_definition: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let msg = HardwareWriteCmd::new(
      &[SATISFYER_PROTOCOL_UUID],
      Endpoint::Command,
      vec![0x01],
      true,
    );
    let info_fut = hardware.write_value(&msg);
    info_fut.await?;

    let feature_count = device_definition
      .features()
      .iter()
      .filter(|x| x.output().is_some())
      .count();

    Ok(Arc::new(Satisfyer::new(feature_count)))
  }
}

pub struct Satisfyer {
  feature_count: usize,
  last_command: Arc<Vec<AtomicU8>>,
}

fn form_command(feature_count: usize, data: Arc<Vec<AtomicU8>>) -> Vec<u8> {
  data[0..feature_count]
    .iter()
    .map(|d| vec![d.load(Ordering::Relaxed); 4])
    .collect::<Vec<Vec<u8>>>()
    .concat()
}

impl Satisfyer {
  fn new(feature_count: usize) -> Self {
    let last_command = Arc::new(
      (0..feature_count)
        .map(|_| AtomicU8::new(0))
        .collect::<Vec<AtomicU8>>(),
    );

    Self {
      feature_count,
      last_command,
    }
  }
}

impl ProtocolHandler for Satisfyer {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(Duration::from_secs(3))
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.last_command[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    let data = form_command(self.feature_count, self.last_command.clone());

    Ok(vec![HardwareWriteCmd::new(
      &[SATISFYER_PROTOCOL_UUID],
      Endpoint::Tx,
      data,
      false,
    )
    .into()])
  }
}
