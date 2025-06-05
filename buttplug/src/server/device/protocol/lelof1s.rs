// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::Endpoint,
  },
  server::{device::{
    configuration::{ProtocolCommunicationSpecifier, UserDeviceDefinition, UserDeviceIdentifier},
    hardware::{Hardware, HardwareCommand, HardwareSubscribeCmd, HardwareWriteCmd},
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
  }, message::checked_actuator_cmd::CheckedActuatorCmdV4},
};
use async_trait::async_trait;
use uuid::{uuid, Uuid};
use std::sync::{atomic::{AtomicU8, Ordering}, Arc};

const LELO_F1S_PROTOCOL_UUID: Uuid = uuid!("4987f232-40f9-47a3-8d0c-e30b74e75310");
generic_protocol_initializer_setup!(LeloF1s, "lelo-f1s");

#[derive(Default)]
pub struct LeloF1sInitializer {}

#[async_trait]
impl ProtocolInitializer for LeloF1sInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    // The Lelo F1s needs you to hit the power button after connection
    // before it'll accept any commands. Unless we listen for event on
    // the button, this is more likely to turn the device off.
    hardware
      .subscribe(&HardwareSubscribeCmd::new(LELO_F1S_PROTOCOL_UUID, Endpoint::Rx))
      .await?;
    Ok(Arc::new(LeloF1s::new(false)))
  }
}

pub struct LeloF1s {
  speeds: [AtomicU8; 2],
  write_with_response: bool
}

impl LeloF1s {
  pub fn new(write_with_response: bool) -> Self {
    Self {
      write_with_response,
      speeds: [AtomicU8::new(0), AtomicU8::new(0)]
    }
  }
}

impl ProtocolHandler for LeloF1s {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn outputs_full_command_set(&self) -> bool {
    true
  }

  fn handle_actuator_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[cmd.feature_index() as usize].store(cmd.value() as u8, Ordering::Relaxed);
    let mut cmd_vec = vec![0x1];
    self.speeds.iter().for_each(|v| cmd_vec.push(v.load(Ordering::Relaxed)));
    Ok(vec![
      HardwareWriteCmd::new(cmd.feature_id(), Endpoint::Tx, cmd_vec, self.write_with_response).into()
    ])
  }
}
