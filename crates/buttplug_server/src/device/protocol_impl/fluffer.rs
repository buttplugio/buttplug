// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareEvent, HardwareSubscribeCmd, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer, ProtocolKeepaliveStrategy},
};
use aes::Aes128;
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::{
  Endpoint,
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};
use ecb::cipher::block_padding::Pkcs7;
use ecb::cipher::{BlockDecryptMut, BlockEncryptMut, KeyInit};
use rand::prelude::*;
use sha2::Digest;
use std::sync::{
  Arc,
  atomic::{AtomicU8, Ordering},
};
use uuid::{Uuid, uuid};

type Aes128EcbEnc = ecb::Encryptor<Aes128>;
type Aes128EcbDec = ecb::Decryptor<Aes128>;

const FLUFFER_PROTOCOL_UUID: Uuid = uuid!("d3721a71-a81d-461a-b404-8599ce50c00b");
const FLUFFER_KEY: [u8; 16] = *b"jdk#Flu%y6fer32f";

pub mod setup {
  use crate::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct FlufferIdentifierFactory {}

  impl ProtocolIdentifierFactory for FlufferIdentifierFactory {
    fn identifier(&self) -> &str {
      "fluffer"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::FlufferIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct FlufferIdentifier {}

#[async_trait]
impl ProtocolIdentifier for FlufferIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    proto: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let mut data: Vec<u8> = vec![];
    if let ProtocolCommunicationSpecifier::BluetoothLE(bt_proto) = proto {
      data = bt_proto
        .manufacturer_data()
        .iter()
        .find(|d| d.company().to_le_bytes().eq(&[0x4au8, 0x68]))
        .map(|d| {
          let mut advertisement = vec![];
          advertisement.extend(d.company().to_le_bytes());
          advertisement.extend(d.data().clone().unwrap_or(vec![]).as_slice());
          advertisement
        })
        .unwrap_or(vec![])
    }
    if data.is_empty() {
      warn!(
        "Failed to get manufacturer data for Fluffer device: {}",
        hardware.name()
      );
    }
    Ok((
      UserDeviceIdentifier::new(
        hardware.address(),
        "fluffer",
        &Some(hardware.name().to_owned()),
      ),
      Box::new(FlufferInitializer::new(data)),
    ))
  }
}

#[derive(Default)]
pub struct FlufferInitializer {
  advertisment_data: Vec<u8>,
}

impl FlufferInitializer {
  fn new(advertisment_data: Vec<u8>) -> Self {
    Self { advertisment_data }
  }
}

fn encrypt(data: Vec<u8>) -> Vec<u8> {
  let enc = Aes128EcbEnc::new(&FLUFFER_KEY.into());
  let res = enc.encrypt_padded_vec_mut::<Pkcs7>(data.as_slice());

  info!("Encoded {:?} to {:?}", data, res);
  res
}

fn decrypt(data: Vec<u8>) -> Vec<u8> {
  let dec = Aes128EcbDec::new(&FLUFFER_KEY.into());
  let res = dec.decrypt_padded_vec_mut::<Pkcs7>(&data).unwrap();

  info!("Decoded {:?} from {:?}", res, data);
  res
}

fn extract_adv(adv_bytes: &[u8]) -> Vec<u8> {
  let key = adv_bytes[2] ^ adv_bytes[1];
  let mut out = vec![];
  let mut src_offset = 0;

  while out.len() < 4 && (3 + src_offset) < adv_bytes.len() {
    let mut b = adv_bytes[3 + src_offset];
    if b != 0 && b != key {
      b ^= key;
    }
    out.push(b);
    src_offset += 1;
  }
  out
}

#[async_trait]
impl ProtocolInitializer for FlufferInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let mut event_receiver = hardware.event_stream();
    hardware
      .subscribe(&HardwareSubscribeCmd::new(
        FLUFFER_PROTOCOL_UUID,
        Endpoint::Rx,
      ))
      .await?;

    if self.advertisment_data.len() > 0 {
      // custom logic to compute 4 bytes from the advBytes
      let adv4bytes = extract_adv(self.advertisment_data.as_slice());

      // random 4 bytes
      let rand: [u8; 4] = [random(), random(), random(), random()];

      // sha256 of the 4 random bytes + the 4 advertisement bytes
      let mut hash = sha2::Sha256::new();
      Digest::update(&mut hash, rand);
      Digest::update(&mut hash, adv4bytes);
      let digest = hash.finalize().to_vec();

      // first 4 bytes from sha256'd data (official app picks a random 4 adjacent bytes)
      let pattern = digest.get(0..4).expect("SHA256 has many bytes...");

      // build full command of [0xA5, 0x0a, 0x08, ...rand 4 bytes, ...first 4 bytes from sha256'd data]
      let mut auth_data = vec![0xa5, 0x01, 0x08];
      auth_data.extend(rand);
      auth_data.extend(pattern);

      hardware
        .write_value(&HardwareWriteCmd::new(
          &[FLUFFER_PROTOCOL_UUID],
          Endpoint::Tx,
          encrypt(auth_data),
          false,
        ))
        .await?;

      loop {
        let event = event_receiver.recv().await;
        return if let Ok(HardwareEvent::Notification(_, _, n)) = event {
          let decoded = decrypt(n);
          if decoded.eq(&vec![0xa5, 0x01, 0x01, 0x00]) {
            debug!("Fluffer authenticated!");

            hardware
              .write_value(&HardwareWriteCmd::new(
                &[FLUFFER_PROTOCOL_UUID],
                Endpoint::Tx,
                encrypt(vec![0x82, 0x0E, 0x02, 0x00, 0x01]),
                false,
              ))
              .await?;

            Ok(Arc::new(Fluffer::default()))
          } else {
            Err(ButtplugDeviceError::ProtocolSpecificError(
              "Fluffer".to_owned(),
              "Fluffer didn't provide a valid security handshake".to_owned(),
            ))
          }
        } else {
          Err(ButtplugDeviceError::ProtocolSpecificError(
            "Fluffer".to_owned(),
            "Fluffer didn't provide a valid security handshake".to_owned(),
          ))
        };
      }
    } else {
      Ok(Arc::new(Fluffer::default()))
    }
  }
}

#[derive(Default)]
pub struct Fluffer {
  speeds: [AtomicU8; 2],
}

impl Fluffer {
  fn send_command(&self) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let s1 = self.speeds[0].fetch_add(0, Ordering::Relaxed);
    let s2 = self.speeds[1].fetch_add(0, Ordering::Relaxed);
    Ok(vec![
      HardwareWriteCmd::new(
        &[FLUFFER_PROTOCOL_UUID],
        Endpoint::Tx,
        encrypt(vec![0x82, 0x0F, 0x05, 0x00, s1, s2, 0x00, 0x00]),
        false,
      )
      .into(),
    ])
  }
}
impl ProtocolHandler for Fluffer {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatPacketStrategy(HardwareWriteCmd::new(
      &[FLUFFER_PROTOCOL_UUID],
      Endpoint::Tx,
      encrypt(vec![0x80, 0x02, 0x00]),
      false,
    ))
  }
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    self.send_command()
  }

  fn handle_output_rotate_cmd(
    &self,
    feature_index: u32,
    _feature_id: uuid::Uuid,
    speed: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(
      if speed < 0 {
        speed.abs() + 100
      } else {
        speed.abs()
      } as u8,
      Ordering::Relaxed,
    );
    self.send_command()
  }

  fn handle_output_oscillate_cmd(
    &self,
    feature_index: u32,
    _feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    self.send_command()
  }
}
