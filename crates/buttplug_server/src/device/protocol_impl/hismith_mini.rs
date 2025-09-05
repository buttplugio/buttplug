// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
};
use async_trait::async_trait;
use buttplug_core::{errors::ButtplugDeviceError, message::OutputType};
use buttplug_server_device_config::{
  Endpoint,
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};
use std::sync::Arc;
use uuid::{Uuid, uuid};

const HISMITH_MINI_PROTOCOL_UUID: Uuid = uuid!("94befc1a-9859-4bf6-99ee-5678c89237a7");
const HISMITH_MINI_ROTATE_DIRECTIOM_UUID: Uuid = uuid!("94befc1a-9859-4bf6-99ee-5678c89237a7");

pub mod setup {
  use crate::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct HismithMiniIdentifierFactory {}

  impl ProtocolIdentifierFactory for HismithMiniIdentifierFactory {
    fn identifier(&self) -> &str {
      "hismith-mini"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::HismithMiniIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct HismithMiniIdentifier {}

#[async_trait]
impl ProtocolIdentifier for HismithMiniIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let result = hardware
      .read_value(&HardwareReadCmd::new(
        HISMITH_MINI_PROTOCOL_UUID,
        Endpoint::RxBLEModel,
        128,
        500,
      ))
      .await?;

    let identifier = result
      .data()
      .iter()
      .map(|b| format!("{b:02x}"))
      .collect::<String>();
    info!("Hismith Device Identifier: {}", identifier);

    Ok((
      UserDeviceIdentifier::new(hardware.address(), "hismith-mini", &Some(identifier)),
      Box::new(HismithMiniInitializer::default()),
    ))
  }
}

#[derive(Default)]
pub struct HismithMiniInitializer {}

#[async_trait]
impl ProtocolInitializer for HismithMiniInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    device_definition: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(HismithMini {
      dual_vibe: device_definition
        .features()
        .iter()
        .filter(|x| {
          x.output()
            .as_ref()
            .is_some_and(|x| x.contains(OutputType::Vibrate))
        })
        .count()
        >= 2,
      second_constrict: device_definition
        .features()
        .iter()
        .position(|x| {
          x.output()
            .as_ref()
            .is_some_and(|x| x.contains(OutputType::Constrict))
        })
        .unwrap_or(0)
        == 1,
    }))
  }
}

#[derive(Default)]
pub struct HismithMini {
  dual_vibe: bool,
  second_constrict: bool,
}

impl ProtocolHandler for HismithMini {
  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let idx: u8 = 0x03;
    let speed: u8 = speed as u8;

    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0xCC, idx, speed, speed + idx],
        false,
      )
      .into(),
    ])
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let idx: u8 = if !self.dual_vibe || feature_index == 1 {
      0x05
    } else {
      0x03
    };
    let speed: u8 = speed as u8;

    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0xCC, idx, speed, speed + idx],
        false,
      )
      .into(),
    ])
  }

  fn handle_output_constrict_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let idx: u8 = if self.second_constrict { 0x05 } else { 0x03 };
    let speed: u8 = level as u8;

    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0xCC, idx, speed, speed + idx],
        false,
      )
      .into(),
    ])
  }

  fn handle_output_rotate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
        Ok(vec![
            HardwareWriteCmd::new(
                &[feature_id],
                Endpoint::Tx,
                vec![0xCC, 0x03, speed as u8, speed as u8 + 3],
                false,
            ).into(),
            HardwareWriteCmd::new(
                &vec![HISMITH_MINI_ROTATE_DIRECTIOM_UUID],
                Endpoint::Tx,
                vec![0xCC, 0x01, if clockwise { 0xc0 } else {0xc1}, if clockwise { 0xc1 } else {0xc2}],
                false,
            ).into(),
        ])
    }

  fn handle_output_spray_cmd(&self, _feature_index: u32, feature_id: Uuid, _level: u32) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![
          0xCC,
          0x03,
          speed.unsigned_abs() as u8,
          speed.unsigned_abs() as u8 + 3,
        ],
        false,
      )
      .into(),
      HardwareWriteCmd::new(
        &vec![HISMITH_MINI_ROTATE_DIRECTIOM_UUID],
        Endpoint::Tx,
        vec![
          0xCC,
          0x01,
          if speed >= 0 { 0xc0 } else { 0xc1 },
          if speed >= 0 { 0xc1 } else { 0xc2 },
        ],
        false,
      )
      .into(),
    ])
  }
}
