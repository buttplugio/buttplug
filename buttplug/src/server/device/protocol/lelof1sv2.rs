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

generic_protocol_initializer_setup!(LeloF1sV2, "lelo-f1sv2");

#[derive(Default)]
pub struct LeloF1sV2Initializer {}

#[async_trait]
impl ProtocolInitializer for LeloF1sV2Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
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
      .subscribe(&HardwareSubscribeCmd::new(Endpoint::Whitelist))
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
          return Ok(Arc::new(LeloF1sV2::default()));
        } else {
          debug!("Lelo F1s V2 gave us a password: {:?}", n);
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
          "LeloF1sV2".to_owned(),
          "Lelo F1s V2 didn't provided valid security handshake".to_owned(),
        ));
      }
    }
  }
}

#[derive(Default)]
pub struct LeloF1sV2 {}

impl ProtocolHandler for LeloF1sV2 {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut cmd_vec = vec![0x1];
    for cmd in cmds.iter() {
      cmd_vec.push(cmd.expect("LeloF1s should always send all values").1 as u8);
    }
    Ok(vec![
      HardwareWriteCmd::new(Endpoint::Tx, cmd_vec, false).into()
    ])
  }
}
