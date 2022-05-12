// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug::{
  core::ButtplugResultFuture,
  server::device::communication_manager::{
    DeviceCommunicationEvent,
    DeviceCommunicationManager,
    DeviceCommunicationManagerBuilder,
  },
};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};
use tokio::sync::mpsc::Sender;

#[derive(Default)]
pub struct DelayDeviceCommunicationManagerBuilder {
  sender: Option<tokio::sync::mpsc::Sender<DeviceCommunicationEvent>>,
}

impl DeviceCommunicationManagerBuilder for DelayDeviceCommunicationManagerBuilder {
  fn event_sender(mut self, sender: Sender<DeviceCommunicationEvent>) -> Self {
    self.sender = Some(sender);
    self
  }

  fn finish(mut self) -> Box<dyn DeviceCommunicationManager> {
    Box::new(DelayDeviceCommunicationManager::new(
      self.sender.take().expect("Test, assuming infallible"),
    ))
  }
}

pub struct DelayDeviceCommunicationManager {
  sender: Sender<DeviceCommunicationEvent>,
  is_scanning: Arc<AtomicBool>,
}

impl DelayDeviceCommunicationManager {
  fn new(sender: Sender<DeviceCommunicationEvent>) -> Self {
    Self {
      sender,
      is_scanning: Arc::new(AtomicBool::new(false)),
    }
  }
}

impl DeviceCommunicationManager for DelayDeviceCommunicationManager {
  fn name(&self) -> &'static str {
    "DelayDeviceCommunicationManager"
  }

  fn start_scanning(&self) -> ButtplugResultFuture {
    let is_scanning = self.is_scanning.clone();
    Box::pin(async move {
      is_scanning.store(true, Ordering::SeqCst);
      Ok(())
    })
  }

  fn stop_scanning(&self) -> ButtplugResultFuture {
    let is_scanning = self.is_scanning.clone();
    let sender = self.sender.clone();
    Box::pin(async move {
      is_scanning.store(false, Ordering::SeqCst);
      sender
        .send(DeviceCommunicationEvent::ScanningFinished)
        .await
        .expect("Test, assuming infallible");
      Ok(())
    })
  }

  fn scanning_status(&self) -> Arc<AtomicBool> {
    self.is_scanning.clone()
  }

  fn can_scan(&self) -> bool {
    true
  }
}
