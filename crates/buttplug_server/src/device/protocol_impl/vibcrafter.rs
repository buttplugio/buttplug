// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareEvent, HardwareSubscribeCmd, HardwareWriteCmd},
  protocol::{
    generic_protocol_initializer_setup,
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
  },
};
use aes::Aes128;
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};
use ecb::cipher::block_padding::Pkcs7;
use ecb::cipher::{BlockDecryptMut, BlockEncryptMut, KeyInit};
use std::sync::{
  atomic::{AtomicU8, Ordering},
  Arc,
};
use uuid::{uuid, Uuid};

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use regex::Regex;
use sha2::{Digest, Sha256};

type Aes128EcbEnc = ecb::Encryptor<Aes128>;
type Aes128EcbDec = ecb::Decryptor<Aes128>;

const VIBCRAFTER_PROTOCOL_UUID: Uuid = uuid!("d3721a71-a81d-461a-b404-8599ce50c00b");
const VIBCRAFTER_KEY: [u8; 16] = *b"jdk#Cra%f5Vib28r";

generic_protocol_initializer_setup!(VibCrafter, "vibcrafter");

#[derive(Default)]
pub struct VibCrafterInitializer {}

fn encrypt(command: String) -> Vec<u8> {
  let enc = Aes128EcbEnc::new(&VIBCRAFTER_KEY.into());
  let res = enc.encrypt_padded_vec_mut::<Pkcs7>(command.as_bytes());

  info!("Encoded {} to {:?}", command, res);
  res
}

fn decrypt(data: Vec<u8>) -> String {
  let dec = Aes128EcbDec::new(&VIBCRAFTER_KEY.into());
  let res = String::from_utf8(dec.decrypt_padded_vec_mut::<Pkcs7>(&data).unwrap()).unwrap();

  info!("Decoded {} from {:?}", res, data);
  res
}

#[async_trait]
impl ProtocolInitializer for VibCrafterInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let mut event_receiver = hardware.event_stream();
    hardware
      .subscribe(&HardwareSubscribeCmd::new(
        VIBCRAFTER_PROTOCOL_UUID,
        Endpoint::Rx,
      ))
      .await?;

    let auth_str = thread_rng()
      .sample_iter(&Alphanumeric)
      .take(6)
      .map(char::from)
      .collect::<String>();
    let auth_msg = format!("Auth:{};", auth_str);
    hardware
      .write_value(&HardwareWriteCmd::new(
        &[VIBCRAFTER_PROTOCOL_UUID],
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
          debug!("VibCrafter authenticated!");
          return Ok(Arc::new(VibCrafter::default()));
        }
        let challenge = Regex::new(r"^[a-zA-Z0-9]{4}:([a-zA-Z0-9]+);$")
          .expect("This is static and should always compile");
        if let Some(parts) = challenge.captures(decoded.as_str()) {
          debug!("VibCrafter challenge {:?}", parts);
          if let Some(to_hash) = parts.get(1) {
            debug!("VibCrafter to hash {:?}", to_hash);
            let mut sha256 = Sha256::new();
            sha256.update(to_hash.as_str().as_bytes());
            let result = &sha256.finalize();

            let auth_msg = format!("Auth:{:02x}{:02x};", result[0], result[1]);
            hardware
              .write_value(&HardwareWriteCmd::new(
                &[VIBCRAFTER_PROTOCOL_UUID],
                Endpoint::Tx,
                encrypt(auth_msg),
                false,
              ))
              .await?;
          } else {
            return Err(ButtplugDeviceError::ProtocolSpecificError(
              "VibCrafter".to_owned(),
              "VibCrafter didn't provide a valid security handshake".to_owned(),
            ));
          }
        } else {
          return Err(ButtplugDeviceError::ProtocolSpecificError(
            "VibCrafter".to_owned(),
            "VibCrafter didn't provide a valid security handshake".to_owned(),
          ));
        }
      } else {
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "VibCrafter".to_owned(),
          "VibCrafter didn't provide a valid security handshake".to_owned(),
        ));
      }
    }
  }
}

#[derive(Default)]
pub struct VibCrafter {
  speeds: [AtomicU8; 2],
}

impl ProtocolHandler for VibCrafter {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);

    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      encrypt(format!(
        "MtInt:{:02}{:02};",
        self.speeds[0].load(Ordering::Relaxed),
        self.speeds[1].load(Ordering::Relaxed)
      )),
      false,
    )
    .into()])
  }
}
