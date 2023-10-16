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
    message::{self, Endpoint},
  },
  server::device::{
    configuration::ProtocolAttributesType,
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{
      fleshlight_launch_helper::calculate_speed,
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
    ServerDeviceIdentifier,
  },
};
use async_trait::async_trait;
use std::sync::{
  atomic::{AtomicU8, Ordering},
  Arc,
};

generic_protocol_initializer_setup!(KiirooV2, "kiiroo-v2");

#[derive(Default)]
pub struct KiirooV2Initializer {}

#[async_trait]
impl ProtocolInitializer for KiirooV2Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let msg = HardwareWriteCmd::new(Endpoint::Firmware, vec![0x0u8], true);
    hardware.write_value(&msg).await?;
    Ok(Arc::new(KiirooV2::default()))
  }
}

#[derive(Default)]
pub struct KiirooV2 {
  previous_position: Arc<AtomicU8>,
}

impl ProtocolHandler for KiirooV2 {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_linear_cmd(
    &self,
    message: message::LinearCmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let v = message.vectors()[0].clone();
    // In the protocol, we know max speed is 99, so convert here. We have to
    // use AtomicU8 because there's no AtomicF64 yet.
    let previous_position = self.previous_position.load(Ordering::SeqCst);
    let distance = (previous_position as f64 - (v.position() * 99f64)).abs() / 99f64;
    let fl_cmd = message::FleshlightLaunchFW12Cmd::new(
      0,
      (v.position() * 99f64) as u8,
      (calculate_speed(distance, v.duration()) * 99f64) as u8,
    );
    self.handle_fleshlight_launch_fw12_cmd(fl_cmd)
  }

  fn handle_fleshlight_launch_fw12_cmd(
    &self,
    message: message::FleshlightLaunchFW12Cmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let position = message.position();
    self.previous_position.store(position, Ordering::SeqCst);
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      [message.position(), message.speed()].to_vec(),
      false,
    )
    .into()])
  }
}
