// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::ButtplugResultFuture;
use buttplug_server::device::hardware::communication::{
  HardwareCommunicationManager,
  HardwareCommunicationManagerBuilder,
  HardwareCommunicationManagerEvent,
};
use futures::FutureExt;
use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};
use tokio::sync::mpsc::Sender;

#[derive(Default)]
pub struct DelayDeviceCommunicationManagerBuilder {}

impl HardwareCommunicationManagerBuilder for DelayDeviceCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(DelayDeviceCommunicationManager::new(sender))
  }
}

pub struct DelayDeviceCommunicationManager {
  sender: Sender<HardwareCommunicationManagerEvent>,
  is_scanning: Arc<AtomicBool>,
}

impl DelayDeviceCommunicationManager {
  fn new(sender: Sender<HardwareCommunicationManagerEvent>) -> Self {
    Self {
      sender,
      is_scanning: Arc::new(AtomicBool::new(false)),
    }
  }
}

impl HardwareCommunicationManager for DelayDeviceCommunicationManager {
  fn name(&self) -> &'static str {
    "DelayDeviceCommunicationManager"
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    let is_scanning = self.is_scanning.clone();
    async move {
      is_scanning.store(true, Ordering::Relaxed);
      Ok(())
    }
    .boxed()
  }

  fn stop_scanning(&mut self) -> ButtplugResultFuture {
    let is_scanning = self.is_scanning.clone();
    let sender = self.sender.clone();
    async move {
      is_scanning.store(false, Ordering::Relaxed);
      sender
        .send(HardwareCommunicationManagerEvent::ScanningFinished)
        .await
        .expect("Test, assuming infallible");
      Ok(())
    }
    .boxed()
  }

  fn scanning_status(&self) -> bool {
    self.is_scanning.load(Ordering::Relaxed)
  }

  fn can_scan(&self) -> bool {
    true
  }
}
