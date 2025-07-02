// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{
    generic_protocol_initializer_setup,
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
  },
};
use async_trait::async_trait;
use buttplug_core::{errors::ButtplugDeviceError, util::sleep};
use buttplug_server_device_config::{
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
  Endpoint,
};
use std::{sync::Arc, time::Duration};
use uuid::{uuid, Uuid};

generic_protocol_initializer_setup!(CowgirlCone, "cowgirl-cone");

const COWGIRL_CONE_PROTOCOL_UUID: Uuid = uuid!("3054b443-eca7-41a6-8ba1-b93a646636a4");

#[derive(Default)]
pub struct CowgirlConeInitializer {}

#[async_trait]
impl ProtocolInitializer for CowgirlConeInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    hardware
      .write_value(&HardwareWriteCmd::new(
        &[COWGIRL_CONE_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![0xaa, 0x56, 0x00, 0x00],
        false,
      ))
      .await?;
    sleep(Duration::from_millis(3000)).await;
    Ok(Arc::new(CowgirlCone::default()))
  }
}

#[derive(Default)]
pub struct CowgirlCone {}

impl ProtocolHandler for CowgirlCone {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![0xf1, 0x01, speed as u8, 0x00],
      false,
    )
    .into()])
  }
}
