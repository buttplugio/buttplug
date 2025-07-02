// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{
  lelo_harmony::LeloHarmony,
  lelof1s::LeloF1s,
};
use crate::device::{
  hardware::{
    Hardware,
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
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};
use std::sync::Arc;
use uuid::{uuid, Uuid};

const LELO_F1S_V2_PROTOCOL_UUID: Uuid = uuid!("85c59ac5-89ee-4549-8958-ce5449226a5c");
generic_protocol_initializer_setup!(LeloF1sV2, "lelo-f1sv2");

#[derive(Default)]
pub struct LeloF1sV2Initializer {}

#[async_trait]
impl ProtocolInitializer for LeloF1sV2Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let use_harmony = !hardware.endpoints().contains(&Endpoint::Whitelist);
    let sec_endpoint = if use_harmony {
      Endpoint::Generic0
    } else {
      Endpoint::Whitelist
    };

    // The Lelo F1s V2 has a very specific pairing flow:
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
      .subscribe(&HardwareSubscribeCmd::new(
        LELO_F1S_V2_PROTOCOL_UUID,
        sec_endpoint,
      ))
      .await?;
    let noauth: Vec<u8> = vec![0; 8];
    let authed: Vec<u8> = vec![1, 0, 0, 0, 0, 0, 0, 0];

    loop {
      let event = event_receiver.recv().await;
      if let Ok(HardwareEvent::Notification(_, _, n)) = event {
        if n.eq(&noauth) {
          info!(
            "Lelo F1s V2 isn't authorised: Tap the device's power button to complete connection."
          )
        } else if n.eq(&authed) {
          debug!("Lelo F1s V2 is authorised!");
          if use_harmony {
            return Ok(Arc::new(LeloHarmony::default()));
          } else {
            return Ok(Arc::new(LeloF1s::new(true)));
          }
        } else {
          debug!("Lelo F1s V2 gave us a password: {:?}", n);
          // Can't send whilst subscribed
          hardware
            .unsubscribe(&HardwareUnsubscribeCmd::new(
              LELO_F1S_V2_PROTOCOL_UUID,
              sec_endpoint,
            ))
            .await?;
          // Send with response
          hardware
            .write_value(&HardwareWriteCmd::new(
              &[LELO_F1S_V2_PROTOCOL_UUID],
              sec_endpoint,
              n,
              true,
            ))
            .await?;
          // Get back to the loop
          hardware
            .subscribe(&HardwareSubscribeCmd::new(
              LELO_F1S_V2_PROTOCOL_UUID,
              sec_endpoint,
            ))
            .await?;
        }
      } else {
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "LeloF1sV2".to_owned(),
          "Lelo F1s V2 didn't provided valid security handshake".to_owned(),
        ));
      }
    }
  }
}
