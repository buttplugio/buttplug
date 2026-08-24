// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::lelo_harmony::LeloHarmony;
use crate::device::{
  hardware::{
    Hardware, HardwareEvent, HardwareSubscribeCmd, HardwareUnsubscribeCmd, HardwareWriteCmd,
  },
  protocol::{
    ProtocolHandler, ProtocolIdentifier, ProtocolInitializer, generic_protocol_initializer_setup,
  },
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::{
  Endpoint, ProtocolCommunicationSpecifier, ServerDeviceDefinition, UserDeviceIdentifier,
};
use std::sync::Arc;
use uuid::{Uuid, uuid};

const LELO_F1S_V3_PROTOCOL_UUID: Uuid = uuid!("f786e955-8295-4ac6-af47-852e4487a1f4");
generic_protocol_initializer_setup!(LeloF1sV3, "lelo-f1sv3");

#[derive(Default)]
pub struct LeloF1sV3Initializer {}

#[async_trait]
impl ProtocolInitializer for LeloF1sV3Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    def: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let mut event_receiver = hardware.event_stream();
    hardware
      .subscribe(&HardwareSubscribeCmd::new(
        LELO_F1S_V3_PROTOCOL_UUID,
        Endpoint::Generic0,
      ))
      .await?;
    let noauth: Vec<u8> = vec![0; 8];
    let authed: Vec<u8> = vec![1, 0, 0, 0, 0, 0, 0, 0];

    loop {
      let event = event_receiver.recv().await;
      if let Ok(HardwareEvent::Notification(_, _, n)) = event {
        if n.eq(&noauth) {
          info!(
            "Lelo F1s V3 isn't authorised: Tap the device's power button to complete connection."
          )
        } else if n.eq(&authed) {
          debug!("Lelo F1s V3 is authorised!");
          return Ok(Arc::new(LeloHarmony::f1sv3(hardware.clone(), def)));
        } else {
          debug!("Lelo F1s V3 gave us a password: {:?}", n);
          hardware
            .unsubscribe(&HardwareUnsubscribeCmd::new(
              LELO_F1S_V3_PROTOCOL_UUID,
              Endpoint::Generic0,
            ))
            .await?;
          hardware
            .write_value(&HardwareWriteCmd::new(
              &[LELO_F1S_V3_PROTOCOL_UUID],
              Endpoint::Generic0,
              n,
              true,
            ))
            .await?;
          hardware
            .subscribe(&HardwareSubscribeCmd::new(
              LELO_F1S_V3_PROTOCOL_UUID,
              Endpoint::Generic0,
            ))
            .await?;
        }
      } else {
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "LeloF1sV3".to_owned(),
          "Lelo F1s V3 didn't provided valid security handshake".to_owned(),
        ));
      }
    }
  }
}
