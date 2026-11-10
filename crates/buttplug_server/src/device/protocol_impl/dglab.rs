// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_trait::async_trait;
use std::sync::{
  Arc,
  atomic::{AtomicU8, Ordering},
};
use std::time::Duration;
use uuid::{Uuid, uuid};

use crate::{
  device::{
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
  },
  generic_protocol_initializer_setup,
};
use buttplug_core::{errors::ButtplugDeviceError, util::async_manager};
use buttplug_server_device_config::{
  Endpoint,
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};

const DGLAB_PROTOCOL_UUID: Uuid = uuid!("3621e028-1af9-4bb3-8524-bded1fa5dc6a");
const DGLAB_DISPALY_UUID: Uuid = uuid!("70b44cf0-d77c-4b4f-8427-07ee5d5409f9");
const DGLAB_WAVEFORM_UUID: Uuid = uuid!("77a75680-92f2-4fda-b2b0-c5599384e660");
const DGLAB_WAVEFORM_INTERVAL: u64 = 100;

generic_protocol_initializer_setup!(DGLab, "dglab");

async fn dglab_waveform(device: Arc<Hardware>) {
  loop {
    if device
      .write_value(&HardwareWriteCmd::new(
        &[DGLAB_WAVEFORM_UUID],
        Endpoint::Tx,
        vec![
          0xb0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x64, 0x64, 0x64, 0x00, 0x00, 0x00,
          0x00, 0x64, 0x64, 0x64, 0x64,
        ],
        false,
      ))
      .await
      .is_err()
    {
      return;
    }
    async_manager::sleep(Duration::from_millis(DGLAB_WAVEFORM_INTERVAL)).await;
  }
}

#[derive(Default)]
pub struct DGLabInitializer {}

#[async_trait]
impl ProtocolInitializer for DGLabInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let msg = HardwareWriteCmd::new(
      &[DGLAB_PROTOCOL_UUID],
      Endpoint::Tx,
      vec![0x50, 0x01, 0x00],
      false,
    );
    hardware.write_value(&msg).await?;

    buttplug_core::spawn!("DGLabWaveform", dglab_waveform(hardware.clone(),));
    Ok(Arc::new(DGLab::default()))
  }
}

pub struct DGLab {
  speeds: [AtomicU8; 2],
}

impl Default for DGLab {
  fn default() -> Self {
    Self {
      speeds: [AtomicU8::new(0), AtomicU8::new(0)],
    }
  }
}

impl ProtocolHandler for DGLab {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);

    let s1 = self.speeds[0].load(Ordering::Relaxed);
    let s2 = self.speeds[1].load(Ordering::Relaxed);

    Ok(vec![
      HardwareWriteCmd::new(
        &[DGLAB_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![0xb3, s1, s2],
        false,
      )
      .into(),
      HardwareWriteCmd::new(
        &[DGLAB_DISPALY_UUID],
        Endpoint::Tx,
        vec![
          0xb2, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
          0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x08, 0x09, s1, s2,
        ],
        false,
      )
      .into(),
    ])
  }
}
