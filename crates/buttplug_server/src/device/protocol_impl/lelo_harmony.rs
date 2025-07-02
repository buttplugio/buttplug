// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
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

const LELO_HARMONY_PROTOCOL_UUID: Uuid = uuid!("220e180a-e6d5-4fd1-963e-43a6f990b717");
generic_protocol_initializer_setup!(LeloHarmony, "lelo-harmony");

#[derive(Default)]
pub struct LeloHarmonyInitializer {}

#[async_trait]
impl ProtocolInitializer for LeloHarmonyInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
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
      .subscribe(&HardwareSubscribeCmd::new(
        LELO_HARMONY_PROTOCOL_UUID,
        Endpoint::Whitelist,
      ))
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
            .unsubscribe(&HardwareUnsubscribeCmd::new(
              LELO_HARMONY_PROTOCOL_UUID,
              Endpoint::Whitelist,
            ))
            .await?;
          // Send with response
          hardware
            .write_value(&HardwareWriteCmd::new(
              &[LELO_HARMONY_PROTOCOL_UUID],
              Endpoint::Whitelist,
              n,
              true,
            ))
            .await?;
          // Get back to the loop
          hardware
            .subscribe(&HardwareSubscribeCmd::new(
              LELO_HARMONY_PROTOCOL_UUID,
              Endpoint::Whitelist,
            ))
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

impl LeloHarmony {
  fn handle_input_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![
        0x0a,
        0x12,
        feature_index as u8 + 1,
        0x08,
        0x00,
        0x00,
        0x00,
        0x00,
        speed as u8,
        0x00,
      ],
      false,
    )
    .into()])
  }
}

impl ProtocolHandler for LeloHarmony {
  fn handle_output_rotate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_input_cmd(feature_index, feature_id, speed)
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_input_cmd(feature_index, feature_id, speed)
  }
}
