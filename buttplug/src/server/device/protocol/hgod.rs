// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::ActuatorType;
use crate::server::device::configuration::ProtocolDeviceAttributes;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
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
use std::{
  sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
  },
  time::Duration,
};

// Time between Hgod update commands, in milliseconds.
const HGOD_COMMAND_DELAY_MS: u64 = 100;

generic_protocol_initializer_setup!(Hgod, "hgod");

#[derive(Default)]
pub struct HgodInitializer {}

#[async_trait]
impl ProtocolInitializer for HgodInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(Hgod::new(hardware)))
  }
}

pub struct Hgod {
  last_command: Arc<AtomicU8>,
}

impl Hgod {
  fn new(hardware: Arc<Hardware>) -> Self {
    let last_command = Arc::new(AtomicU8::new(0));

    let last_command_clone = last_command.clone();
    async_manager::spawn(async move {
      send_hgod_updates(hardware, last_command_clone).await;
    });

    Self { last_command }
  }
}

// HGod toys vibes only last ~100ms seconds.
async fn send_hgod_updates(device: Arc<Hardware>, data: Arc<AtomicU8>) {
  loop {
    let speed = data.load(Ordering::SeqCst);
    let command = vec![0x55, 0x04, 0, 0, 0, speed];
    if speed > 0 {
      if let Err(e) = device
        .write_value(&HardwareWriteCmd::new(Endpoint::Tx, command, false))
        .await
      {
        error!(
          "Got an error from a hgod device, exiting control loop: {:?}",
          e
        );
        break;
      }
    }
    sleep(Duration::from_millis(HGOD_COMMAND_DELAY_MS)).await;
  }
}

impl ProtocolHandler for Hgod {
  fn handle_scalar_cmd(
    &self,
    commands: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if let Some(cmd) = commands[0] {
      self.last_command.store(cmd.1 as u8, Ordering::SeqCst);
    }
    Ok(vec![])
  }
}
