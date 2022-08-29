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
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let msg = HardwareWriteCmd::new(Endpoint::Command, vec![0x01], true);
    let info_fut = hardware.write_value(&msg);

    info_fut.await?;
    Ok(Arc::new(Satisfyer::new(hardware)))
  }
}

pub struct Satisfyer {
  last_command: Arc<[AtomicU8; 2]>,
}

fn form_command(command1: u8, command2: u8) -> Vec<u8> {
  [[command1; 4], [command2; 4]].concat()
}

// Satisfyer toys will drop their connections if they don't get an update within ~10 seconds.
// Therefore we try to send a command every ~1s unless something is sent/updated sooner.
async fn send_satisfyer_updates(device: Arc<Hardware>, data: Arc<[AtomicU8; 2]>) {
  loop {
    let command_val_0 = data[0].load(Ordering::SeqCst);
    let command_val_1 = data[1].load(Ordering::SeqCst);
    let command = form_command(command_val_0, command_val_1);
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
    let last_command = Arc::new([AtomicU8::new(0), AtomicU8::new(0)]);
    let last_command_clone = last_command.clone();
    async_manager::spawn(async move {
      send_satisfyer_updates(hardware, last_command_clone).await;
    });

    Self { last_command }
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
    let data = if commands.len() == 1 {
      let command_val = commands[0].as_ref().unwrap().1 as u8;
      self.last_command[0].store(command_val, Ordering::SeqCst);
      form_command(command_val, 0)
    } else {
      // These end up flipped for some reason.
      let command_val_0 = commands[1].as_ref().unwrap().1 as u8;
      let command_val_1 = commands[0].as_ref().unwrap().1 as u8;
      self.last_command[0].store(command_val_0, Ordering::SeqCst);
      self.last_command[1].store(command_val_1, Ordering::SeqCst);
      form_command(command_val_0, command_val_1)
    };
    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, false).into()])
  }
}
