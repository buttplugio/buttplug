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

generic_protocol_initializer_setup!(SvakomDT250A, "svakom-dt250a");

#[derive(Default)]
pub struct SvakomDT250AInitializer {}

#[async_trait]
impl ProtocolInitializer for SvakomDT250AInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(SvakomDT250A::new(hardware)))
  }
}

async fn delayed_update_handler(device: Arc<Hardware>, cmd: Vec<u8>, delay: u64) {
  sleep(Duration::from_millis(delay)).await;
  let res = device
    .write_value(&HardwareWriteCmd::new(Endpoint::Tx, cmd, false))
    .await;
  if res.is_err() {
    error!("Delayed Svakom DT250A command error: {:?}", res.err());
  }
}

pub struct SvakomDT250A {
  device: Arc<Hardware>,
}
impl SvakomDT250A {
  fn new(device: Arc<Hardware>) -> Self {
    Self { device }
  }
}

impl ProtocolHandler for SvakomDT250A {
  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if cmds.len() == 0 {
      return Ok(vec![]);
    }

    let mut delay = 30;
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
          scalar as u8,
          if scalar == 0 { 0x00 } else { 0x01 },
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
      let data = [
        0x55,
        0x08,
        0x00,
        0x00,
        scalar as u8,
        if scalar == 0 { 0x00 } else { 0x01 },
      ]
      .to_vec();

      if hcmd.is_none() {
        return Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, false).into()]);
      } else {
        // Sending both commands in quick succession blots the earlier command
        let dev = self.device.clone();
        async_manager::spawn(async move { delayed_update_handler(dev, data, delay).await });

        // This is the minimum time between the 2nd and 3rd command that doesn't seem to just get dropped...
        delay += 250;
      }
    }

    if cmds.len() < 3 {
      return if hcmd.is_some() {
        Ok(vec![hcmd.unwrap().into()])
      } else {
        Ok(vec![])
      };
    }

    if let Some(cmd) = cmds[2] {
      let scalar = cmd.1;
      let data = [0x55, 0x09, 0x00, 0x00, scalar as u8, 0x00].to_vec();

      if hcmd.is_none() {
        return Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, false).into()]);
      } else {
        // Sending both commands in quick succession blots the earlier command
        let dev = self.device.clone();
        async_manager::spawn(async move { delayed_update_handler(dev, data, delay).await });
      }
    }

    return if hcmd.is_some() {
      Ok(vec![hcmd.unwrap().into()])
    } else {
      Ok(vec![])
    };
  }
}
