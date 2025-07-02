// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::{
  atomic::{AtomicU8, Ordering},
  Arc,
};

use async_trait::async_trait;
use uuid::{uuid, Uuid};

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{
    generic_protocol_initializer_setup,
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
  },
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{DeviceDefinition, ProtocolCommunicationSpecifier, UserDeviceIdentifier};
const LOVEHONEY_DESIRE_PROTOCOL_UUID: Uuid = uuid!("5dcd8487-4814-44cb-a768-13bf81d545c0");
const LOVEHONEY_DESIRE_VIBE2_PROTOCOL_UUID: Uuid = uuid!("d44a99fe-903b-4fff-bee7-1141767c9cca");

generic_protocol_initializer_setup!(LovehoneyDesire, "lovehoney-desire");

#[derive(Default)]
pub struct LovehoneyDesireInitializer {}

#[async_trait]
impl ProtocolInitializer for LovehoneyDesireInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    def: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(LovehoneyDesire::new(
      def
        .features()
        .iter()
        .filter(|x| x.output().is_some())
        .count() as u8,
    )))
  }
}

pub struct LovehoneyDesire {
  current_commands: Vec<AtomicU8>,
}

impl LovehoneyDesire {
  fn new(num_vibrators: u8) -> Self {
    Self {
      current_commands: std::iter::repeat_with(|| AtomicU8::default())
        .take(num_vibrators as usize)
        .collect(),
    }
  }
}

impl ProtocolHandler for LovehoneyDesire {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // The Lovehoney Desire has 2 types of commands
    //
    // - Set both motors with one command
    // - Set each motor separately
    //
    // We'll need to check what we got back and write our
    // commands accordingly.
    if self.current_commands.len() == 1 {
      Ok(vec![HardwareWriteCmd::new(
        &[LOVEHONEY_DESIRE_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![0xF3, 0, speed as u8],
        true,
      )
      .into()])
    } else {
      self.current_commands[feature_index as usize].store(speed as u8, Ordering::Relaxed);
      let speed0 = self.current_commands[0].load(Ordering::Relaxed);
      let speed1 = self.current_commands[1].load(Ordering::Relaxed);
      if speed0 == speed1 {
        Ok(vec![HardwareWriteCmd::new(
          &[
            LOVEHONEY_DESIRE_PROTOCOL_UUID,
            LOVEHONEY_DESIRE_VIBE2_PROTOCOL_UUID,
          ],
          Endpoint::Tx,
          vec![0xF3, 0, speed0 as u8],
          true,
        )
        .into()])
      } else {
        Ok(vec![
          HardwareWriteCmd::new(
            &[LOVEHONEY_DESIRE_PROTOCOL_UUID],
            Endpoint::Tx,
            vec![0xF3, 1, speed0 as u8],
            true,
          )
          .into(),
          HardwareWriteCmd::new(
            &[LOVEHONEY_DESIRE_VIBE2_PROTOCOL_UUID],
            Endpoint::Tx,
            vec![0xF3, 2, speed1 as u8],
            true,
          )
          .into(),
        ])
      }
    }
  }
}
