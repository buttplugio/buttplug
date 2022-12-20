// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::configuration::ProtocolDeviceAttributes;
use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::device::{
    configuration::ProtocolAttributesType,
    hardware::{
      Hardware,
      HardwareCommand,
      HardwareEvent,
      HardwareSubscribeCmd,
      HardwareUnsubscribeCmd,
      HardwareWriteCmd,
    },
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
    ServerDeviceIdentifier,
  },
};
use async_trait::async_trait;
use std::sync::Arc;

generic_protocol_initializer_setup!(LeloHarmony, "lelo-harmony");

#[derive(Default)]
pub struct LeloHarmonyInitializer {}

#[async_trait]
impl ProtocolInitializer for LeloHarmonyInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    // The Lelo Harmony has a very specific pairing flow:
    // * First the device is turned on in BLE mode (long press)
    // * Then the security endpoint (Whitelist) needs to be read (which we can do via subscribe)
    // * If it returns 0x00,00,00,00,00,00,00,00 the connection isn't not authorised
    // * To authorize, the password must be writen to the characteristic.
    // * If the password is unknown (buttplug lacks a storage mechanism right now), the power button
    //   must be pressed to send the password
    // * The password must not be sent whilst subscribed to the endpoint
    // * Once the password has been sent, the endpoint can be read for status again
    // * If it returns 0x00,00,00,00,00,00,00,00 the connection is authorised
    let mut event_receiver = hardware.event_stream();
    hardware
      .subscribe(&HardwareSubscribeCmd::new(Endpoint::Whitelist))
      .await?;

    loop {
      let event = event_receiver.recv().await;
      if let Ok(HardwareEvent::Notification(_, _, n)) = event {
        if n.iter().all(|b| *b == 0u8) {
          info!(
            "Lelo Harmony isn't authorised: Tap the device's power button to complete connection."
          )
        } else if !n.is_empty() && n[0] == 1u8 && n[1..].iter().all(|b| *b == 0u8) {
          debug!("Lelo Harmony is authorised!");
          return Ok(Arc::new(LeloHarmony::default()));
        } else {
          debug!("Lelo Harmony gave us a password: {:?}", n);
          // Can't send whilst subscribed
          hardware
            .unsubscribe(&HardwareUnsubscribeCmd::new(Endpoint::Whitelist))
            .await?;
          // Send with response
          hardware
            .write_value(&HardwareWriteCmd::new(Endpoint::Whitelist, n, true))
            .await?;
          // Get back to the loop
          hardware
            .subscribe(&HardwareSubscribeCmd::new(Endpoint::Whitelist))
            .await?;
        }
      } else {
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "LeloHarmony".to_owned(),
          "Lelo Harmony didn't provided valid security handshake".to_owned(),
        ));
      }
    }
  }
}

#[derive(Default)]
pub struct LeloHarmony {}

impl ProtocolHandler for LeloHarmony {
  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut cmd_vec: Vec<HardwareCommand> = vec![];
    for (i, cmd) in cmds.iter().enumerate() {
      if let Some(pair) = cmd {
        cmd_vec.push(
          HardwareWriteCmd::new(
            Endpoint::Tx,
            vec![
              0x0a,
              0x12,
              i as u8 + 1,
              0x08,
              0x00,
              0x00,
              0x00,
              0x00,
              pair.1 as u8,
              0x00,
            ],
            false,
          )
          .into(),
        );
      }
    }
    Ok(cmd_vec)
  }
}
