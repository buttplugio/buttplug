// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::{
    device::{
      configuration::{ProtocolCommunicationSpecifier, UserDeviceDefinition, UserDeviceIdentifier},
      hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
      protocol::{
        fleshlight_launch_helper::calculate_speed,
        generic_protocol_initializer_setup,
        ProtocolHandler,
        ProtocolIdentifier,
        ProtocolInitializer,
      },
    },
    message::{checked_actuator_cmd::CheckedActuatorCmdV4, checked_value_with_parameter_cmd::CheckedValueWithParameterCmdV4, FleshlightLaunchFW12CmdV0},
  },
};
use async_trait::async_trait;
use uuid::{uuid, Uuid};
use std::sync::{
  atomic::{AtomicU8, Ordering},
  Arc,
};

const KIIROO_V21_INITIALIZED_PROTOCOL_UUID: Uuid = uuid!("22329023-5464-41b6-a0de-673d7e993055");

generic_protocol_initializer_setup!(KiirooV21Initialized, "kiiroo-v21-initialized");

#[derive(Default)]
pub struct KiirooV21InitializedInitializer {}

#[async_trait]
impl ProtocolInitializer for KiirooV21InitializedInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    debug!("calling Onyx+ init");
    hardware
      .write_value(&HardwareWriteCmd::new(
        KIIROO_V21_INITIALIZED_PROTOCOL_UUID,
        Endpoint::Tx,
        vec![0x03u8, 0x00u8, 0x64u8, 0x19u8],
        true,
      ))
      .await?;
    hardware
      .write_value(&HardwareWriteCmd::new(
        KIIROO_V21_INITIALIZED_PROTOCOL_UUID,
        Endpoint::Tx,
        vec![0x03u8, 0x00u8, 0x64u8, 0x00u8],
        true,
      ))
      .await?;
    Ok(Arc::new(KiirooV21Initialized::default()))
  }
}

#[derive(Default)]
pub struct KiirooV21Initialized {
  previous_position: Arc<AtomicU8>,
}

impl KiirooV21Initialized {
  fn handle_fleshlight_launch_fw12_cmd(
    &self,
    uuid: Uuid,
    message: FleshlightLaunchFW12CmdV0,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let position = message.position();
    self.previous_position.store(position, Ordering::Relaxed);
    Ok(vec![HardwareWriteCmd::new(
      uuid,
      Endpoint::Tx,
      [0x03, 0x00, message.speed(), message.position()].to_vec(),
      false,
    )
    .into()])
  }
}

impl ProtocolHandler for KiirooV21Initialized {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_actuator_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      cmd.feature_id(),
      Endpoint::Tx,
      vec![0x01, cmd.value() as u8],
      false,
    )
    .into()])
  }

  fn handle_position_with_duration_cmd(
    &self,
    cmd: &CheckedValueWithParameterCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // In the protocol, we know max speed is 99, so convert here. We have to
    // use AtomicU8 because there's no AtomicF64 yet.
    let previous_position = self.previous_position.load(Ordering::Relaxed);
    let distance = (previous_position as f64 - (cmd.value() as f64)).abs() / 99f64;
    let fl_cmd = FleshlightLaunchFW12CmdV0::new(
      0,
      cmd.value() as u8,
      (calculate_speed(distance, cmd.parameter() as u32) * 99f64) as u8,
    );
    self.handle_fleshlight_launch_fw12_cmd(cmd.feature_id(), fl_cmd)
  }

}
