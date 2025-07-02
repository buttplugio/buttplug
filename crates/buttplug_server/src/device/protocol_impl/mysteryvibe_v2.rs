// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::mysteryvibe::MysteryVibe;
use crate::device::{
  hardware::{Hardware, HardwareWriteCmd},
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

generic_protocol_initializer_setup!(MysteryVibeV2, "mysteryvibe-v2");

const MYSTERYVIBE_V2_PROTOCOL_UUID: Uuid = uuid!("215a2c34-11fa-419a-84d2-60ac6acbc9f8");

#[derive(Default)]
pub struct MysteryVibeV2Initializer {}

#[async_trait]
impl ProtocolInitializer for MysteryVibeV2Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    def: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    // The only thing that's different about MysteryVibeV2 from v1 is the initialization packet.
    // Just send that then return the older protocol version.
    let msg = HardwareWriteCmd::new(
      &[MYSTERYVIBE_V2_PROTOCOL_UUID],
      Endpoint::TxMode,
      vec![0x03u8, 0x02u8, 0x40u8],
      true,
    );
    hardware.write_value(&msg).await?;
    let vibrator_count = def
      .features()
      .iter()
      .filter(|x| x.output().is_some())
      .count();
    Ok(Arc::new(MysteryVibe::new(vibrator_count as u8)))
  }
}
