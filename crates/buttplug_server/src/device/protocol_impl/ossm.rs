// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.


use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{
    ProtocolHandler, 
    ProtocolIdentifier,
    ProtocolInitializer, 
    generic_protocol_initializer_setup
  },
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::{
  Endpoint,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
  ProtocolCommunicationSpecifier,
};
use std::sync::Arc;
use uuid::{Uuid, uuid};
use async_trait::async_trait;

const OSSM_PROTOCOL_UUID: Uuid = uuid!("a817e40d-acda-439d-bebf-420badbabe69");
generic_protocol_initializer_setup!(OSSM, "ossm");

#[derive(Default)]
pub struct OSSMInitializer {}

#[async_trait]
impl ProtocolInitializer for OSSMInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let msg = HardwareWriteCmd::new(
      &[OSSM_PROTOCOL_UUID],
      Endpoint::Tx,
      format!("go:strokeEngine").into_bytes(),
      false,
    );
    hardware.write_value(&msg).await?;
    Ok(Arc::new(OSSM::default()))
  }
}

#[derive(Default)]
pub struct OSSM {}

impl ProtocolHandler for OSSM {
  fn handle_output_oscillate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    value: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let param = if feature_index == 0 {
      "speed"
    } else {
      return Err(ButtplugDeviceError::DeviceFeatureMismatch(
        format!("OSSM command received for unknown feature index: {}", feature_index),
      ));
    };
    
    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        format!("set:{param}:{value}").into_bytes(),
        false,
      )
      .into(),
    ])
  }
}
