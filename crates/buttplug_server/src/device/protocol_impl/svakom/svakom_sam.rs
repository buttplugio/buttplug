// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::protocol::ProtocolKeepaliveStrategy;
use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareSubscribeCmd, HardwareWriteCmd},
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

generic_protocol_initializer_setup!(SvakomSam, "svakom-sam");
const SVAKOM_SAM_PROTOCOL_UUID: Uuid = uuid!("e39a6b4a-230a-4669-be94-68135f97f166");

#[derive(Default)]
pub struct SvakomSamInitializer {}

#[async_trait]
impl ProtocolInitializer for SvakomSamInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    hardware
      .subscribe(&HardwareSubscribeCmd::new(
        SVAKOM_SAM_PROTOCOL_UUID,
        Endpoint::Rx,
      ))
      .await?;
    let mut gen2 = hardware.endpoints().contains(&Endpoint::TxMode);
    if !gen2 && hardware.endpoints().contains(&Endpoint::Firmware) {
      gen2 = true;
      warn!("Svakom Sam model without speed control detected - This device will only vibrate at 1 speed");
    }

    Ok(Arc::new(SvakomSam::new(gen2)))
  }
}

pub struct SvakomSam {
  gen2: bool,
}

impl SvakomSam {
  pub fn new(gen2: bool) -> Self {
    Self { gen2 }
  }
}

impl ProtocolHandler for SvakomSam {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if feature_index == 0 {
      Ok(vec![HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        if self.gen2 {
          [
            18,
            1,
            3,
            0,
            if speed == 0 { 0x00 } else { 0x04 },
            speed as u8,
          ]
          .to_vec()
        } else {
          [18, 1, 3, 0, 5, speed as u8].to_vec()
        },
        false,
      )
      .into()])
    } else {
      Ok(vec![HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        [18, 6, 1, speed as u8].to_vec(),
        false,
      )
      .into()])
    }
  }
}
