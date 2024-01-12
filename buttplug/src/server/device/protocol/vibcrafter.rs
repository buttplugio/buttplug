// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::ActuatorType;
use crate::server::device::configuration::ProtocolDeviceAttributes;
use crate::server::device::hardware::{HardwareEvent, HardwareSubscribeCmd};
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    configuration::ProtocolAttributesType,
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
    ServerDeviceIdentifier,
  },
};
use aes::Aes128;
use async_trait::async_trait;
use ecb::cipher::block_padding::Pkcs7;
use ecb::cipher::{BlockDecryptMut, BlockEncryptMut, KeyInit};
use std::sync::Arc;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use regex::Regex;
use sha2::{Digest, Sha256};

type Aes128EcbEnc = ecb::Encryptor<Aes128>;
type Aes128EcbDec = ecb::Decryptor<Aes128>;

const VIBCRAFTER_KEY: [u8; 16] = *b"jdk#Cra%f5Vib28r";

generic_protocol_initializer_setup!(VibCrafter, "vibcrafter");

#[derive(Default)]
pub struct VibCrafterInitializer {}

fn encrypt(command: String) -> Vec<u8> {
  let enc = Aes128EcbEnc::new(&VIBCRAFTER_KEY.into());
  let res = enc.encrypt_padded_vec_mut::<Pkcs7>(command.as_bytes());

  info!("Encoded {} to {:?}", command, res);
  return res;
}

fn decrypt(data: Vec<u8>) -> String {
  let dec = Aes128EcbDec::new(&VIBCRAFTER_KEY.into());
  let res = String::from_utf8(dec.decrypt_padded_vec_mut::<Pkcs7>(&data).unwrap()).unwrap();

  info!("Decoded {} from {:?}", res, data);
  return res;
}

#[async_trait]
impl ProtocolInitializer for VibCrafterInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let mut event_receiver = hardware.event_stream();
    hardware
      .subscribe(&HardwareSubscribeCmd::new(Endpoint::Rx))
      .await?;

    let auth_str = thread_rng()
      .sample_iter(&Alphanumeric)
      .take(6)
      .map(char::from)
      .collect::<String>();
    let auth_msg = format!("Auth:{};", auth_str);
    hardware
      .write_value(&HardwareWriteCmd::new(
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
            sha256.update(&to_hash.as_str().as_bytes());
            let result = &sha256.finalize();

            let auth_msg = format!("Auth:{:02x}{:02x};", result[0], result[1]);
            hardware
              .write_value(&HardwareWriteCmd::new(
                Endpoint::Tx,
                encrypt(auth_msg),
                false,
              ))
              .await?;
          } else {
            return Err(ButtplugDeviceError::ProtocolSpecificError(
              "VibCrafter".to_owned(),
              "VibCrafter didn't provided valid security handshake".to_owned(),
            ));
          }
        } else {
          return Err(ButtplugDeviceError::ProtocolSpecificError(
            "VibCrafter".to_owned(),
            "VibCrafter didn't provided valid security handshake".to_owned(),
          ));
        }
      } else {
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "VibCrafter".to_owned(),
          "VibCrafter didn't provided valid security handshake".to_owned(),
        ));
      }
    }
  }
}

#[derive(Default)]
pub struct VibCrafter {}

impl ProtocolHandler for VibCrafter {
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
    let speed0 = commands[0].unwrap_or((ActuatorType::Vibrate, 0)).1;
    let speed1 = if commands.len() > 1 {
      commands[1].unwrap_or((ActuatorType::Vibrate, 0)).1
    } else {
      speed0
    };

    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      encrypt(format!("MtInt:{:02}{:02};", speed0, speed1)),
      false,
    )
    .into()])
  }
}
