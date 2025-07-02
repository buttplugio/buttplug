// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};
use std::sync::{
  atomic::{AtomicU8, Ordering},
  Arc,
};
use uuid::{uuid, Uuid};

pub mod setup {
  use crate::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct MonsterPubIdentifierFactory {}

  impl ProtocolIdentifierFactory for MonsterPubIdentifierFactory {
    fn identifier(&self) -> &str {
      "monsterpub"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::MonsterPubIdentifier::default())
    }
  }
}

const MONSTERPUB_PROTOCOL_UUID: Uuid = uuid!("c7fe6c69-e7c2-4fa9-822a-6bb337dece1a");

#[derive(Default)]
pub struct MonsterPubIdentifier {}

#[async_trait]
impl ProtocolIdentifier for MonsterPubIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let read_resp = hardware
      .read_value(&HardwareReadCmd::new(
        MONSTERPUB_PROTOCOL_UUID,
        Endpoint::RxBLEModel,
        32,
        500,
      ))
      .await;
    let ident = match read_resp {
      Ok(data) => std::str::from_utf8(data.data())
        .map_err(|_| {
          ButtplugDeviceError::ProtocolSpecificError(
            "monsterpub".to_owned(),
            "MonsterPub device name is non-UTF8 string.".to_owned(),
          )
        })?
        .replace("\0", "")
        .to_owned(),
      Err(_) => "Unknown".to_string(),
    };
    return Ok((
      UserDeviceIdentifier::new(hardware.address(), "monsterpub", &Some(ident)),
      Box::new(MonsterPubInitializer::default()),
    ));
  }
}

#[derive(Default)]
pub struct MonsterPubInitializer {}

#[async_trait]
impl ProtocolInitializer for MonsterPubInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    def: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    if hardware.endpoints().contains(&Endpoint::Rx) {
      let value = hardware
        .read_value(&HardwareReadCmd::new(
          MONSTERPUB_PROTOCOL_UUID,
          Endpoint::Rx,
          16,
          200,
        ))
        .await?;
      let keys = [
        [
          0x32u8, 0x49, 0x50, 0x4f, 0x32, 0x49, 0x50, 0x4f, 0x32, 0x49, 0x50, 0x4f, 0x32, 0x49,
          0x50,
        ],
        [
          0x4c, 0x53, 0x42, 0x42, 0x4c, 0x53, 0x42, 0x42, 0x4c, 0x53, 0x42, 0x42, 0x4c, 0x53, 0x42,
        ],
        [
          0x53, 0x49, 0x53, 0x36, 0x53, 0x49, 0x53, 0x36, 0x53, 0x49, 0x53, 0x36, 0x53, 0x49, 0x53,
        ],
        [
          0x54, 0x41, 0x4c, 0x4b, 0x54, 0x41, 0x4c, 0x4b, 0x54, 0x41, 0x4c, 0x4b, 0x54, 0x41, 0x4c,
        ],
      ];

      let auth = value.data()[1..16]
        .iter()
        .zip(keys[value.data()[0] as usize].iter())
        .map(|(&x1, &x2)| x1 ^ x2)
        .collect();

      trace!(
        "Got {:?} XOR with key {} to get {:?}",
        value.data(),
        value.data()[0],
        auth
      );

      hardware
        .write_value(&HardwareWriteCmd::new(
          &[MONSTERPUB_PROTOCOL_UUID],
          Endpoint::Rx,
          auth,
          true,
        ))
        .await?;
    }
    let output_count = def
      .features()
      .iter()
      .filter(|x| x.output().is_some())
      .count();

    Ok(Arc::new(MonsterPub::new(
      if hardware.endpoints().contains(&Endpoint::TxVibrate) {
        Endpoint::TxVibrate
      } else if hardware.endpoints().contains(&Endpoint::Tx) {
        Endpoint::Tx
      } else {
        Endpoint::Generic0 // tracy's dog 3 vibe
      },
      output_count as u32,
    )))
  }
}

pub struct MonsterPub {
  tx: Endpoint,
  speeds: Vec<AtomicU8>,
}

impl MonsterPub {
  pub fn new(tx: Endpoint, num_outputs: u32) -> Self {
    let speeds: Vec<AtomicU8> = std::iter::repeat_with(|| AtomicU8::default())
      .take(num_outputs as usize)
      .collect();
    Self { tx, speeds }
  }

  fn form_command(&self) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut data = vec![];
    let mut stop = true;

    if self.tx == Endpoint::Generic0 {
      data.push(3u8);
    }
    for cmd in self.speeds.iter() {
      let speed = cmd.load(Ordering::Relaxed);
      data.push(speed as u8);
      if speed != 0 {
        stop = false;
      }
    }
    let tx = if self.tx == Endpoint::Tx && stop {
      Endpoint::TxMode
    } else {
      self.tx
    };
    Ok(vec![HardwareWriteCmd::new(
      &[MONSTERPUB_PROTOCOL_UUID],
      tx,
      data,
      tx == Endpoint::TxMode,
    )
    .into()])
  }
}

impl ProtocolHandler for MonsterPub {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    self.form_command()
  }

  fn handle_output_oscillate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    self.form_command()
  }
}
