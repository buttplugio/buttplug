// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::configuration::ProtocolDeviceAttributes;
use crate::util::async_manager;
use crate::{
  core::{errors::ButtplugDeviceError, message, message::Endpoint},
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
  util::sleep,
};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

generic_protocol_initializer_setup!(LongLostTouch, "longlosttouch");

#[derive(Default)]
pub struct LongLostTouchInitializer {}

#[async_trait]
impl ProtocolInitializer for LongLostTouchInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _attributes: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(LongLostTouch::new(hardware)))
  }
}

pub struct LongLostTouch {
  last_command: Arc<Vec<AtomicU8>>,
}

fn form_commands(data: Arc<Vec<AtomicU8>>, force: Option<Vec<bool>>) -> Vec<Vec<u8>> {
  let mut cmds: Vec<Vec<u8>> = Vec::new();
  if data.len() != 2 {
    return cmds;
  }

  let mut skip = vec![false; data.len()];
  let mut zero = vec![false; data.len()];
  if let Some(f) = force {
    if f.len() != 2 {
      return cmds;
    }
    for (i, force) in f.iter().enumerate() {
      if !force {
        skip[i] = true;
      } else {
        zero[i] = true;
      }
    }
  }

  if data[0].load(Ordering::SeqCst) == data[1].load(Ordering::SeqCst) {
    if zero[0] || zero[1] || data[0].load(Ordering::SeqCst) != 0 {
      cmds.push(vec![
        0xAA,
        0x02,
        0x00,
        0x00,
        0x00,
        data[0].load(Ordering::SeqCst),
      ])
    }
    return cmds;
  }

  (0..2).for_each(|i| {
    if !skip[i as usize] && (zero[i as usize] || data[i as usize].load(Ordering::SeqCst) != 0) {
      cmds.push(vec![
        0xAA,
        0x02,
        i + 1 as u8,
        0x00,
        0x00,
        data[i as usize].load(Ordering::SeqCst),
      ])
    }
  });
  return cmds;
}

async fn send_longlosttouch_updates(device: Arc<Hardware>, data: Arc<Vec<AtomicU8>>) {
  loop {
    let cmds = form_commands(data.clone(), None);
    for cmd in cmds {
      if let Err(e) = device
        .write_value(&HardwareWriteCmd::new(Endpoint::Tx, cmd, true).into())
        .await
      {
        error!(
          "Got an error from a long lost touch device, exiting control loop: {:?}",
          e
        );
        break;
      }
    }
    sleep(Duration::from_millis(2500)).await;
  }
}

impl LongLostTouch {
  fn new(hardware: Arc<Hardware>) -> Self {
    let last_command = Arc::new((0..2).map(|_| AtomicU8::new(0)).collect::<Vec<AtomicU8>>());
    let last_command_clone = last_command.clone();
    async_manager::spawn(async move {
      send_longlosttouch_updates(hardware, last_command_clone).await;
    });

    Self { last_command }
  }
}

impl ProtocolHandler for LongLostTouch {
  fn handle_scalar_cmd(
    &self,
    commands: &[Option<(message::ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if commands.len() != 2 {
      return Err(ButtplugDeviceError::DeviceFeatureCountMismatch(
        2,
        commands.len() as u32,
      ));
    }
    for (i, item) in commands.iter().enumerate() {
      if let Some(command) = item {
        self.last_command[i].store(command.1 as u8, Ordering::SeqCst);
      }
    }
    Ok(
      form_commands(
        self.last_command.clone(),
        Some(commands.iter().map(|i| i.is_some()).collect()),
      )
      .iter()
      .map(|data| HardwareWriteCmd::new(Endpoint::Tx, data.clone(), true).into())
      .collect(),
    )
  }
}
