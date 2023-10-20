// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::btleplug_adapter_task::{BtleplugAdapterCommand, BtleplugAdapterTask};
use crate::{
  core::{errors::ButtplugDeviceError, ButtplugResultFuture},
  server::device::hardware::communication::{
    HardwareCommunicationManager,
    HardwareCommunicationManagerBuilder,
    HardwareCommunicationManagerEvent,
  },
  util::async_manager,
};
use futures::future::FutureExt;
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};
use tokio::sync::mpsc::{channel, Sender};

#[derive(Default, Clone)]
pub struct BtlePlugCommunicationManagerBuilder {
  require_keepalive: bool,
}

impl BtlePlugCommunicationManagerBuilder {
  pub fn requires_keepalive(&mut self, require: bool) -> &mut Self {
    self.require_keepalive = require;
    self
  }
}

impl HardwareCommunicationManagerBuilder for BtlePlugCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(BtlePlugCommunicationManager::new(
      sender,
      self.require_keepalive,
    ))
  }
}

pub struct BtlePlugCommunicationManager {
  adapter_event_sender: Sender<BtleplugAdapterCommand>,
  scanning_status: Arc<AtomicBool>,
  adapter_connected: Arc<AtomicBool>,
}

impl BtlePlugCommunicationManager {
  pub fn new(
    event_sender: Sender<HardwareCommunicationManagerEvent>,
    require_keepalive: bool,
  ) -> Self {
    let (sender, receiver) = channel(256);
    let adapter_connected = Arc::new(AtomicBool::new(false));
    let adapter_connected_clone = adapter_connected.clone();
    async_manager::spawn(async move {
      let mut task = BtleplugAdapterTask::new(
        event_sender,
        receiver,
        adapter_connected_clone,
        require_keepalive,
      );
      task.run().await;
    });
    Self {
      adapter_event_sender: sender,
      scanning_status: Arc::new(AtomicBool::new(false)),
      adapter_connected,
    }
  }
}

impl HardwareCommunicationManager for BtlePlugCommunicationManager {
  fn name(&self) -> &'static str {
    "BtlePlugCommunicationManager"
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    let adapter_event_sender = self.adapter_event_sender.clone();
    let scanning_status = self.scanning_status.clone();
    // Set to true just to make sure we don't call ScanningFinished too early.
    scanning_status.store(true, Ordering::SeqCst);
    async move {
      if adapter_event_sender
        .send(BtleplugAdapterCommand::StartScanning)
        .await
        .is_err()
      {
        error!("Error starting scan, cannot send to btleplug event loop.");
        scanning_status.store(false, Ordering::SeqCst);
        Err(
          ButtplugDeviceError::DeviceConnectionError(
            "Cannot send start scanning request to event loop.".to_owned(),
          )
          .into(),
        )
      } else {
        Ok(())
      }
    }
    .boxed()
  }

  fn stop_scanning(&mut self) -> ButtplugResultFuture {
    let adapter_event_sender = self.adapter_event_sender.clone();
    // Just assume any outcome of this means we're done scanning.
    self.scanning_status.store(false, Ordering::SeqCst);
    async move {
      if adapter_event_sender
        .send(BtleplugAdapterCommand::StopScanning)
        .await
        .is_err()
      {
        error!("Error stopping scan, cannot send to btleplug event loop.");
        Err(
          ButtplugDeviceError::DeviceConnectionError(
            "Cannot send stop scanning request to event loop.".to_owned(),
          )
          .into(),
        )
      } else {
        Ok(())
      }
    }
    .boxed()
  }

  fn scanning_status(&self) -> bool {
    self.scanning_status.load(Ordering::SeqCst)
  }

  fn can_scan(&self) -> bool {
    self.adapter_connected.load(Ordering::SeqCst)
  }
}
/*
impl Drop for BtlePlugCommunicationManager {
  fn drop(&mut self) {
    info!("Dropping btleplug comm manager.");
    if self.adapter.is_some() {
      if let Err(e) = self.adapter.as_ref().expect("Already checked validity").stop_scan() {
        info!("Error on scanning shutdown for bluetooth: {:?}", e);
      }
    }
  }
}
 */
