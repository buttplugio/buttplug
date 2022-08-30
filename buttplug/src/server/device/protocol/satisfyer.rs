// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

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
  util::async_manager,
};
use async_trait::async_trait;
use std::{
  sync::{
    atomic::{AtomicU8, AtomicUsize, Ordering},
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
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let msg = HardwareWriteCmd::new(Endpoint::Command, vec![0x01], true);
    let info_fut = hardware.write_value(&msg);

    info_fut.await?;
    Ok(Arc::new(Satisfyer::new(hardware)))
  }
}

pub struct Satisfyer {
  // The largest feature count is currently 5
  feature_count: AtomicUsize,
  last_command: Arc<[AtomicU8; 5]>,
}

fn form_command(feature_count: usize, data: Arc<[AtomicU8; 5]>) -> Vec<u8> {
  data[0..feature_count]
      .iter()
      .map( |d| vec![d.load(Ordering::SeqCst); 4] )
      .collect::<Vec<Vec<u8>>>()
      .concat()
}

// Satisfyer toys will drop their connections if they don't get an update within ~10 seconds.
// Therefore we try to send a command every ~1s unless something is sent/updated sooner.
async fn send_satisfyer_updates(device: Arc<Hardware>, feature_count: usize, data: Arc<[AtomicU8; 5]>) {
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
    tokio::time::sleep(Duration::from_secs(1)).await;
  }
}

impl Satisfyer {
  fn new(hardware: Arc<Hardware>) -> Self {
    let last_command = Arc::new([
      AtomicU8::new(0),
      AtomicU8::new(0),
      AtomicU8::new(0),
      AtomicU8::new(0),
      AtomicU8::new(0)
    ]);
    let last_command_clone = last_command.clone();
    //XXX: Ideally this would be driven off of the attributes, but that would require passing
    //     attrs into protocol_initializer.initialize(), which is a larger change than I want to
    //     make without discussing it first
    let feature_count = 2;
    async_manager::spawn(async move {
      send_satisfyer_updates(hardware, feature_count, last_command_clone).await;
    });

    Self { feature_count: AtomicUsize::new(feature_count), last_command }
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
    let count = commands.len();
    self.feature_count.store(commands.len(), Ordering::SeqCst);

    for i in 0..count {
      let command_val = commands[i].as_ref().unwrap().1 as u8;
      self.last_command[i].store(command_val, Ordering::SeqCst);
    }
    let data = form_command(self.feature_count.load(Ordering::SeqCst), self.last_command.clone());

    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, false).into()])
  }
}
