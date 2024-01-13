// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
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
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
    ServerDeviceIdentifier,
  },
  util::{async_manager, sleep},
};
use async_trait::async_trait;
use std::{sync::Arc, time::Duration};

generic_protocol_initializer_setup!(SvakomSuitcase, "svakom-suitcase");

#[derive(Default)]
pub struct SvakomSuitcaseInitializer {}

#[async_trait]
impl ProtocolInitializer for SvakomSuitcaseInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(SvakomSuitcase::new(hardware)))
  }
}

async fn delayed_update_handler(device: Arc<Hardware>, scalar: u8) {
  sleep(Duration::from_millis(50)).await;
  let res = device
    .write_value(&HardwareWriteCmd::new(
      Endpoint::Tx,
      [0x55, 0x09, 0x00, 0x00, scalar as u8, 0x00].to_vec(),
      false,
    ))
    .await;
  if res.is_err() {
    error!("Delayed Svakom Suitcase command error: {:?}", res.err());
  }
}

pub struct SvakomSuitcase {
  device: Arc<Hardware>,
}
impl SvakomSuitcase {
  fn new(device: Arc<Hardware>) -> Self {
    Self { device }
  }
}

impl ProtocolHandler for SvakomSuitcase {
  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if cmds.len() == 0 {
      return Ok(vec![]);
    }

    let mut hcmd = None;
    if let Some(cmd) = cmds[0] {
      let scalar = cmd.1;
      let mut speed = (scalar % 10) as u8;
      let mut intensity = if scalar == 0 {
        0u8
      } else {
        (scalar as f32 / 10.0).floor() as u8 + 1
      };
      if speed == 0 && intensity != 0 {
        // 10 -> 2,0 -> 1,A
        speed = 10;
        intensity -= 1;
      }

      hcmd = Some(HardwareWriteCmd::new(
        Endpoint::Tx,
        [0x55, 0x03, 0x00, 0x00, intensity, speed].to_vec(),
        false,
      ));
    }

    if cmds.len() < 2 {
      return if hcmd.is_some() {
        Ok(vec![hcmd.unwrap().into()])
      } else {
        Ok(vec![])
      };
    }

    if let Some(cmd) = cmds[1] {
      let scalar = cmd.1;

      if hcmd.is_none() {
        return Ok(vec![HardwareWriteCmd::new(
          Endpoint::Tx,
          [0x55, 0x09, 0x00, 0x00, scalar as u8, 0x00].to_vec(),
          false,
        )
        .into()]);
      } else {
        // Sending both commands in quick succession blots the earlier command
        let dev = self.device.clone();
        async_manager::spawn(async move { delayed_update_handler(dev, scalar as u8).await });
      }
    }

    return if hcmd.is_some() {
      Ok(vec![hcmd.unwrap().into()])
    } else {
      Ok(vec![])
    };
  }
}
