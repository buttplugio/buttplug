// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
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
};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

generic_protocol_initializer_setup!(SvakomIker, "svakom-iker");

#[derive(Default)]
pub struct SvakomIkerInitializer {}

#[async_trait]
impl ProtocolInitializer for SvakomIkerInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(SvakomIker::new()))
  }
}

#[derive(Default)]
pub struct SvakomIker {
  last_speeds: Arc<Vec<AtomicU8>>,
}

impl SvakomIker {
  fn new() -> Self {
    let last_speeds = Arc::new((0..2).map(|_| AtomicU8::new(0)).collect::<Vec<AtomicU8>>());

    Self { last_speeds }
  }
}

impl ProtocolHandler for SvakomIker {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut vibe_off = false;
    let mut msg_vec = vec![];
    if let Some((_, speed)) = cmds[0] {
      self.last_speeds[0].store(speed as u8, Ordering::SeqCst);
      if speed == 0 {
        vibe_off = true;
      }
      msg_vec.push(
        HardwareWriteCmd::new(
          Endpoint::Tx,
          [0x55, 0x03, 0x03, 0x00, 0x01, speed as u8].to_vec(),
          false,
        )
        .into(),
      );
    }
    if cmds.len() > 1 {
      if let Some((_, speed)) = cmds[1] {
        self.last_speeds[1].store(speed as u8, Ordering::SeqCst);
        msg_vec.push(
          HardwareWriteCmd::new(
            Endpoint::Tx,
            [0x55, 0x07, 0x00, 0x00, speed as u8, 0x00].to_vec(),
            false,
          )
          .into(),
        );
      } else if vibe_off && self.last_speeds[1].load(Ordering::SeqCst) != 0 {
        msg_vec.push(
          HardwareWriteCmd::new(
            Endpoint::Tx,
            [
              0x55,
              0x07,
              0x00,
              0x00,
              self.last_speeds[1].load(Ordering::SeqCst),
              0x00,
            ]
            .to_vec(),
            false,
          )
          .into(),
        );
      }
    }
    Ok(msg_vec)
  }
}
