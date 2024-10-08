// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::configuration::ProtocolCommunicationSpecifier;
use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::device::{
    configuration::{UserDeviceDefinition, UserDeviceIdentifier},
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
  },
  util::{async_manager, sleep},
};
use async_trait::async_trait;
use std::{sync::Arc, time::Duration};

generic_protocol_initializer_setup!(SvakomTaraX, "svakom-tarax");

#[derive(Default)]
pub struct SvakomTaraXInitializer {}

#[async_trait]
impl ProtocolInitializer for SvakomTaraXInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(SvakomTaraX::new(hardware)))
  }
}

async fn delayed_update_handler(device: Arc<Hardware>, scalar: u8) {
  sleep(Duration::from_millis(25)).await;
  let res = device
    .write_value(&HardwareWriteCmd::new(
      Endpoint::Tx,
      [0x55, 0x09, 0x00, 0x00, scalar as u8, 0x00].to_vec(),
      false,
    ))
    .await;
  if res.is_err() {
    error!("Delayed Svakom Tara X command error: {:?}", res.err());
  }
}

pub struct SvakomTaraX {
  device: Arc<Hardware>,
}
impl SvakomTaraX {
  fn new(device: Arc<Hardware>) -> Self {
    Self { device }
  }
}

impl ProtocolHandler for SvakomTaraX {
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
      hcmd = Some(HardwareWriteCmd::new(
        Endpoint::Tx,
        [
          0x55,
          0x03,
          0x00,
          0x00,
          if scalar == 0 { 0x01 } else { scalar as u8 },
          if scalar == 0 { 0x01 } else { 0x02 },
        ]
        .to_vec(),
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
