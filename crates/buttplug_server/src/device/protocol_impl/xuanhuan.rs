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

const XUANHUAN_PROTOCOL_ID: Uuid = uuid!("e9f9f8ab-4fd5-4573-a4ec-ab542568849b");
generic_protocol_initializer_setup!(Xuanhuan, "xuanhuan");

#[derive(Default)]
pub struct XuanhuanInitializer {}

#[async_trait]
impl ProtocolInitializer for XuanhuanInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(Xuanhuan::new(hardware)))
  }
}

async fn vibration_update_handler(device: Arc<Hardware>, command_holder: Arc<AtomicU8>) {
  info!("Entering Xuanhuan Control Loop");
  loop {
    let speed = command_holder.load(Ordering::Relaxed);
    if speed != 0 {
      let current_command = vec![0x03, 0x02, 0x00, speed];
      if device
        .write_value(&HardwareWriteCmd::new(
          &[XUANHUAN_PROTOCOL_ID],
          Endpoint::Tx,
          current_command,
          true,
        ))
        .await
        .is_err()
      {
        break;
      }
    }
    sleep(Duration::from_millis(300)).await;
  }
  info!("Xuanhuan control loop exiting, most likely due to device disconnection.");
}

pub struct Xuanhuan {
  current_command: Arc<AtomicU8>,
}

impl Xuanhuan {
  fn new(device: Arc<Hardware>) -> Self {
    let current_command = Arc::new(AtomicU8::new(0));
    let current_command_clone = current_command.clone();
    async_manager::spawn(
      async move { vibration_update_handler(device, current_command_clone).await },
    );
    Self { current_command }
  }
}

impl ProtocolHandler for Xuanhuan {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let speed = speed as u8;
    self.current_command.store(speed, Ordering::Relaxed);

    Ok(vec![HardwareWriteCmd::new(
      &[XUANHUAN_PROTOCOL_ID],
      Endpoint::Tx,
      vec![0x03, 0x02, 0x00, speed],
      true,
    )
    .into()])
  }
}
