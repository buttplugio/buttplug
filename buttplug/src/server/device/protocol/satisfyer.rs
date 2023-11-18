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
    hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
    protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
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

pub mod setup {
  use crate::server::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct SatisfyerIdentifierFactory {}

  impl ProtocolIdentifierFactory for SatisfyerIdentifierFactory {
    fn identifier(&self) -> &str {
      "satisfyer"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::SatisfyerIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct SatisfyerIdentifier {}

#[async_trait]
impl ProtocolIdentifier for SatisfyerIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<(ServerDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let result = hardware
      .read_value(&HardwareReadCmd::new(Endpoint::RxBLEModel, 128, 500))
      .await?;
    let device_identifier = format!(
      "{}",
      u32::from_be_bytes(result.data().to_vec().try_into().unwrap_or([0; 4]))
    );
    info!(
      "Satisfyer Device Identifier: {:?} {}",
      result.data(),
      device_identifier
    );
    return Ok((
      ServerDeviceIdentifier::new(
        hardware.address(),
        "satisfyer",
        &ProtocolAttributesType::Identifier(device_identifier),
      ),
      Box::new(SatisfyerInitializer::default()),
    ));
  }
}

#[derive(Default)]
pub struct SatisfyerInitializer {}

#[async_trait]
impl ProtocolInitializer for SatisfyerInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    attributes: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let msg = HardwareWriteCmd::new(Endpoint::Command, vec![0x01], true);
    let info_fut = hardware.write_value(&msg);
    info_fut.await?;

    let mut feature_count = 2; // fallback to 2
    if let Some(attrs) = attributes.message_attributes.scalar_cmd() {
      feature_count = attrs.len();
    }
    Ok(Arc::new(Satisfyer::new(hardware, feature_count)))
  }
}

pub struct Satisfyer {
  feature_count: usize,
  last_command: Arc<Vec<AtomicU8>>,
}

fn form_command(feature_count: usize, data: Arc<Vec<AtomicU8>>) -> Vec<u8> {
  data[0..feature_count]
    .iter()
    .map(|d| vec![d.load(Ordering::SeqCst); 4])
    .collect::<Vec<Vec<u8>>>()
    .concat()
}

// Satisfyer toys will drop their connections if they don't get an update within ~10 seconds.
// Therefore we try to send a command every ~1s unless something is sent/updated sooner.
async fn send_satisfyer_updates(
  device: Arc<Hardware>,
  feature_count: usize,
  data: Arc<Vec<AtomicU8>>,
) {
  loop {
    let command = form_command(feature_count, data.clone());
    if let Err(e) = device
      .write_value(&HardwareWriteCmd::new(Endpoint::Tx, command, false))
      .await
    {
      error!(
        "Got an error from a satisfyer device, exiting control loop: {:?}",
        e
      );
      break;
    }
    sleep(Duration::from_secs(1)).await;
  }
}

impl Satisfyer {
  fn new(hardware: Arc<Hardware>, feature_count: usize) -> Self {
    let last_command = Arc::new(
      (0..feature_count)
        .map(|_| AtomicU8::new(0))
        .collect::<Vec<AtomicU8>>(),
    );
    let last_command_clone = last_command.clone();
    async_manager::spawn(async move {
      send_satisfyer_updates(hardware, feature_count, last_command_clone).await;
    });

    Self {
      feature_count,
      last_command,
    }
  }
}

impl ProtocolHandler for Satisfyer {
  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn handle_scalar_cmd(
    &self,
    commands: &[Option<(message::ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if self.feature_count != commands.len() {
      return Err(ButtplugDeviceError::DeviceFeatureCountMismatch(
        self.feature_count as u32,
        commands.len() as u32,
      ));
    }
    for (i, item) in commands.iter().enumerate() {
      let command_val = item.as_ref().unwrap().1 as u8;
      self.last_command[i].store(command_val, Ordering::SeqCst);
    }
    let data = form_command(self.feature_count, self.last_command.clone());

    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, false).into()])
  }
}
