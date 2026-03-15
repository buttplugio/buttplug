// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareEvent, HardwareSubscribeCmd, HardwareWriteCmd},
  protocol::{
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
    generic_protocol_initializer_setup,
  },
};
use aes::Aes128;
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};
use ecb::cipher::block_padding::Pkcs7;
use ecb::cipher::{BlockDecryptMut, BlockEncryptMut, KeyInit};
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};
use uuid::{Uuid, uuid};

use rand::distr::Alphanumeric;
use rand::RngExt;
use regex::Regex;
use sha2::{Digest, Sha256};

type Aes128EcbEnc = ecb::Encryptor<Aes128>;
type Aes128EcbDec = ecb::Decryptor<Aes128>;

const VIBIO_PROTOCOL_UUID: Uuid = uuid!("b8c76c9e-cb42-4a94-99f4-7c2a8e5d3b2a");
const VIBIO_KEY: [u8; 16] = *b"jdk#vib%y5fir21a";

generic_protocol_initializer_setup!(Vibio, "vibio");

#[derive(Default)]
pub struct VibioInitializer {}

fn encrypt(command: String) -> Vec<u8> {
  let enc = Aes128EcbEnc::new(&VIBIO_KEY.into());
  let res = enc.encrypt_padded_vec_mut::<Pkcs7>(command.as_bytes());

  info!("Encoded {} to {:?}", command, res);
  res
}

fn decrypt(data: Vec<u8>) -> String {
  let dec = Aes128EcbDec::new(&VIBIO_KEY.into());
  let res = String::from_utf8(dec.decrypt_padded_vec_mut::<Pkcs7>(&data).unwrap()).unwrap();

  info!("Decoded {} from {:?}", res, data);
  res
}

#[async_trait]
impl ProtocolInitializer for VibioInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let mut event_receiver = hardware.event_stream();
    hardware
      .subscribe(&HardwareSubscribeCmd::new(
        VIBIO_PROTOCOL_UUID,
        Endpoint::Rx,
      ))
      .await?;

    let auth_str = rand::rng()
      .sample_iter(&Alphanumeric)
      .take(8)
      .map(char::from)
      .collect::<String>();
    let auth_msg = format!("Auth:{};", auth_str);
    hardware
      .write_value(&HardwareWriteCmd::new(
        &[VIBIO_PROTOCOL_UUID],
        Endpoint::Tx,
        encrypt(auth_msg),
        false,
      ))
      .await?;

    loop {
      let event = event_receiver.recv().await;
      if let Ok(HardwareEvent::Notification(_, _, n)) = event {
        let decoded = decrypt(n);
        if decoded.eq("OK;") {
          debug!("Vibio authenticated!");
          return Ok(Arc::new(Vibio::default()));
        }
        let challenge = Regex::new(r"^([0-9A-Fa-f]{4}):([^;]+);$")
          .expect("This is static and should always compile");
        if let Some(parts) = challenge.captures(decoded.as_str()) {
          debug!("Vibio challenge {:?}", parts);
          if let Some(to_hash) = parts.get(2) {
            debug!("Vibio to hash {:?}", to_hash);
            let mut sha256 = Sha256::new();
            sha256.update(to_hash.as_str().as_bytes());
            let result = &sha256.finalize();

            let auth_msg = format!("Auth:{:02x}{:02x};", result[0], result[1]);
            hardware
              .write_value(&HardwareWriteCmd::new(
                &[VIBIO_PROTOCOL_UUID],
                Endpoint::Tx,
                encrypt(auth_msg),
                false,
              ))
              .await?;
          } else {
            return Err(ButtplugDeviceError::ProtocolSpecificError(
              "Vibio".to_owned(),
              "Vibio didn't provide a valid security handshake".to_owned(),
            ));
          }
        } else {
          return Err(ButtplugDeviceError::ProtocolSpecificError(
            "Vibio".to_owned(),
            "Vibio didn't provide a valid security handshake".to_owned(),
          ));
        }
      } else {
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "Vibio".to_owned(),
          "Vibio didn't provide a valid security handshake".to_owned(),
        ));
      }
    }
  }
}

#[derive(Default)]
pub struct Vibio {
  speeds: [AtomicU8; 2],
}

impl ProtocolHandler for Vibio {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);

    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        encrypt(format!(
          "MtInt:{:02}{:02};",
          self.speeds[0].load(Ordering::Relaxed),
          self.speeds[1].load(Ordering::Relaxed)
        )),
        false,
      )
      .into(),
    ])
  }
}
