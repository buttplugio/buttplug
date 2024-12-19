// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::device::{
    configuration::{ProtocolCommunicationSpecifier, UserDeviceDefinition, UserDeviceIdentifier},
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
    _: &UserDeviceDefinition,
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
      .subscribe(&HardwareSubscribeCmd::new(sec_endpoint))
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
          return Ok(Arc::new(LeloF1sV2::new(use_harmony)));
        } else {
          debug!("Lelo F1s V2 gave us a password: {:?}", n);
          // Can't send whilst subscribed
          hardware
            .unsubscribe(&HardwareUnsubscribeCmd::new(sec_endpoint))
            .await?;
          // Send with response
          hardware
            .write_value(&HardwareWriteCmd::new(sec_endpoint, n, true))
            .await?;
          // Get back to the loop
          hardware
            .subscribe(&HardwareSubscribeCmd::new(sec_endpoint))
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

pub struct LeloF1sV2 {
  use_harmony: bool,
}

impl LeloF1sV2 {
  fn new(use_harmony: bool) -> Self {
    Self { use_harmony }
  }
}

impl ProtocolHandler for LeloF1sV2 {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn needs_full_command_set(&self) -> bool {
    !self.use_harmony
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if self.use_harmony {
      let mut cmd_vec: Vec<HardwareCommand> = vec![];
      for (i, cmd) in cmds.iter().enumerate() {
        if let Some(pair) = cmd {
          cmd_vec.push(
            HardwareWriteCmd::new(
              Endpoint::TxVibrate,
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
      return Ok(cmd_vec);
    }
    let mut cmd_vec = vec![0x1];
    for cmd in cmds.iter() {
      cmd_vec.push(cmd.expect("LeloF1s should always send all values").1 as u8);
    }
    Ok(vec![
      HardwareWriteCmd::new(Endpoint::Tx, cmd_vec, true).into()
    ])
  }
}
