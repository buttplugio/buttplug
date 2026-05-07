// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::str::from_utf8;
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
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use uuid::{Uuid, uuid};
use async_trait::async_trait;
use futures_util::FutureExt;
use crate::device::hardware::HardwareReadCmd;

const OSSM_PROTOCOL_UUID: Uuid = uuid!("a817e40d-acda-439d-bebf-420badbabe69");
const OSSM_MODE_NONE: u8 = 0;
const OSSM_MODE_OSCILLATE: u8 = 1;
const OSSM_MODE_POSITION: u8 = 2;
generic_protocol_initializer_setup!(OSSM, "ossm");

#[derive(Default)]
pub struct OSSMInitializer {
}

#[async_trait]
impl ProtocolInitializer for OSSMInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(OSSM::new(hardware.clone())))
  }
}

pub struct OSSM {
  mode: AtomicU8,
  hardware: Arc<Hardware>,
}

impl OSSM {
  fn new(hardware: Arc<Hardware>) -> OSSM {
    OSSM { mode: AtomicU8::new(OSSM_MODE_NONE), hardware }
  }
}

impl ProtocolHandler for OSSM {
  fn handle_output_oscillate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    value: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut cmds = vec![];
    if self.mode.load(Ordering::Relaxed) != OSSM_MODE_OSCILLATE {
      cmds.push(HardwareWriteCmd::new(
        &[OSSM_PROTOCOL_UUID],
        Endpoint::Tx,
        "go:menu".to_string().into_bytes(),
        true,
      ).into());
      
      cmds.push(HardwareWriteCmd::new(
          &[OSSM_PROTOCOL_UUID],
          Endpoint::Tx,
          "go:strokeEngine".to_string().into_bytes(),
          true,
        ).into());
        cmds.push(HardwareWriteCmd::new(
          &[OSSM_PROTOCOL_UUID],
          Endpoint::TxMode,
          "false".to_string().into_bytes(),
          true,
        ).into());
        cmds.push(HardwareWriteCmd::new(
          &[OSSM_PROTOCOL_UUID],
          Endpoint::Tx,
          "set:depth:100".to_string().into_bytes(),
          true,
        ).into());
        cmds.push(HardwareWriteCmd::new(
          &[OSSM_PROTOCOL_UUID],
          Endpoint::Tx,
          "set:stroke:100".to_string().into_bytes(),
          true,
        ).into());
      self.mode.store(OSSM_MODE_OSCILLATE, Ordering::Relaxed);
    }

    let param = if feature_index == 0 {
      "speed"
    } else {
      return Err(ButtplugDeviceError::DeviceFeatureMismatch(
        format!("OSSM command received for unknown feature index: {}", feature_index),
      ));
    };
    cmds.push(HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        format!("set:{param}:{value}").into_bytes(),
        true,
      ).into());

    Ok(cmds)
  }

  fn handle_hw_position_with_duration_cmd(&self, _feature_index: u32, feature_id: Uuid, position: u32, duration: u32) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut cmds = vec![];
    if self.mode.load(Ordering::Relaxed) != OSSM_MODE_POSITION {
      cmds.push(HardwareWriteCmd::new(
        &[OSSM_PROTOCOL_UUID],
        Endpoint::Tx,
        "go:menu".to_string().into_bytes(),
        true,
      ).into());
      cmds.push(HardwareWriteCmd::new(
        &[OSSM_PROTOCOL_UUID],
        Endpoint::Tx,
        "go:streaming".to_string().into_bytes(),
        true,
      ).into());
      cmds.push(HardwareWriteCmd::new(
          &[OSSM_PROTOCOL_UUID],
          Endpoint::TxMode,
          "false".to_string().into_bytes(),
          true,
      ).into());
      cmds.push(HardwareWriteCmd::new(
        &[OSSM_PROTOCOL_UUID],
        Endpoint::Tx,
        "set:speed:100".to_string().into_bytes(),
        true,
      ).into());
      cmds.push(HardwareWriteCmd::new(
        &[OSSM_PROTOCOL_UUID],
        Endpoint::Tx,
        "set:depth:100".to_string().into_bytes(),
        true,
      ).into());
      cmds.push(HardwareWriteCmd::new(
          &[OSSM_PROTOCOL_UUID],
          Endpoint::Tx,
          "set:stroke:100".to_string().into_bytes(),
          true,
      ).into());
      self.mode.store(OSSM_MODE_POSITION, Ordering::Relaxed);
    }

    cmds.push(HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        format!("stream:{position}:{duration}").into_bytes(),
        true,
      )
          .into()
    );

    Ok(cmds)
  }
}
