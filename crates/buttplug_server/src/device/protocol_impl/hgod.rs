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
use buttplug_core::{
  errors::ButtplugDeviceError,
  util::{async_manager, sleep},
};
use buttplug_server_device_config::{
  Endpoint,
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};
use std::{
  sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
  },
  time::Duration,
};
use uuid::{uuid, Uuid};

// Time between Hgod update commands, in milliseconds.
const HGOD_COMMAND_DELAY_MS: u64 = 100;

const HGOD_PROTOCOL_UUID: Uuid = uuid!("0a086d5b-9918-4b73-b2dd-86ed66de6f51");
generic_protocol_initializer_setup!(Hgod, "hgod");

#[derive(Default)]
pub struct HgodInitializer {}

#[async_trait]
impl ProtocolInitializer for HgodInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
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
    let speed = data.load(Ordering::Relaxed);
    let command = vec![0x55, 0x04, 0, 0, 0, speed];
    if speed > 0 {
      if let Err(e) = device
        .write_value(&HardwareWriteCmd::new(
          &[HGOD_PROTOCOL_UUID],
          Endpoint::Tx,
          command,
          false,
        ))
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
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.last_command.store(speed as u8, Ordering::Relaxed);
    Ok(vec![])
  }
}
