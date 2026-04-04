// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server::device::hardware::{
  GenericHardwareSpecializer, Hardware, HardwareConnector, HardwareEvent, HardwareInternal,
  HardwareReadCmd, HardwareReading, HardwareSpecializer, HardwareSubscribeCmd,
  HardwareUnsubscribeCmd, HardwareWriteCmd,
};
use buttplug_server_device_config::{Endpoint, ProtocolCommunicationSpecifier};
use dashmap::DashSet;
use futures::future::{self, BoxFuture, FutureExt};
use std::{
  collections::VecDeque,
  fmt::{self, Debug},
  sync::Arc,
};
use tokio::sync::{broadcast, Mutex};

/// A simulated hardware device that captures writes and injects reads for testing
#[derive(Clone)]
pub struct SimulatedDevice {
  name: Arc<String>,
  event_sender: broadcast::Sender<HardwareEvent>,
  write_log: Arc<Mutex<Vec<HardwareWriteCmd>>>,
  read_queue: Arc<Mutex<VecDeque<HardwareReading>>>,
  subscribed_endpoints: Arc<DashSet<Endpoint>>,
  endpoints: Arc<Vec<Endpoint>>,
}

impl SimulatedDevice {
  /// Create a new simulated device
  pub fn new(name: &str, endpoints: Vec<Endpoint>) -> Self {
    let (event_sender, _) = broadcast::channel(256);

    Self {
      name: Arc::new(name.to_owned()),
      event_sender,
      write_log: Arc::new(Mutex::new(Vec::new())),
      read_queue: Arc::new(Mutex::new(VecDeque::new())),
      subscribed_endpoints: Arc::new(DashSet::new()),
      endpoints: Arc::new(endpoints),
    }
  }

  /// Create a new simulated device with shared state (for device manager)
  pub fn new_with_shared_state(
    name: &str,
    endpoints: Vec<Endpoint>,
    write_log: Arc<Mutex<Vec<HardwareWriteCmd>>>,
    read_queue: Arc<Mutex<VecDeque<HardwareReading>>>,
    event_sender: broadcast::Sender<HardwareEvent>,
  ) -> Self {
    Self {
      name: Arc::new(name.to_owned()),
      event_sender,
      write_log,
      read_queue,
      subscribed_endpoints: Arc::new(DashSet::new()),
      endpoints: Arc::new(endpoints),
    }
  }

  /// Get the write log for external inspection
  pub fn write_log(&self) -> Arc<Mutex<Vec<HardwareWriteCmd>>> {
    self.write_log.clone()
  }

  /// Queue a read response
  pub fn queue_read(&self, reading: HardwareReading) {
    // Use blocking_lock to avoid tokio::spawn which can cause race conditions in tests
    self.read_queue.blocking_lock().push_back(reading);
  }

  /// Inject a notification if the endpoint is subscribed
  pub fn inject_notification(&self, endpoint: Endpoint, data: Vec<u8>) {
    if self.subscribed_endpoints.contains(&endpoint) {
      let name = (*self.name).clone();
      let _ = self.event_sender.send(HardwareEvent::Notification(name, endpoint, data));
    }
  }

  /// Get a reference to the event sender for direct event injection
  pub fn event_sender(&self) -> &broadcast::Sender<HardwareEvent> {
    &self.event_sender
  }
}

#[async_trait]
impl HardwareInternal for SimulatedDevice {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.event_sender.subscribe()
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let sender = self.event_sender.clone();
    let name = (*self.name).clone();
    async move {
      let _ = sender.send(HardwareEvent::Disconnected(name));
      Ok(())
    }
    .boxed()
  }

  fn read_value(
    &self,
    msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    let queue = self.read_queue.clone();
    let endpoint = msg.endpoint();

    async move {
      let mut guard = queue.lock().await;
      guard.pop_front().ok_or_else(|| {
        ButtplugDeviceError::DeviceCommunicationError(format!(
          "No read data queued for endpoint {}",
          endpoint
        ))
      })
    }
    .boxed()
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    if !self.endpoints.as_ref().contains(&msg.endpoint()) {
      return future::ready(Err(ButtplugDeviceError::InvalidEndpoint(
        msg.endpoint().to_string(),
      )))
      .boxed();
    }

    let log = self.write_log.clone();
    let msg_clone = msg.clone();

    async move {
      log.lock().await.push(msg_clone);
      Ok(())
    }
    .boxed()
  }

  fn subscribe(
    &self,
    msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    if !self.endpoints.as_ref().contains(&msg.endpoint()) {
      return future::ready(Err(ButtplugDeviceError::InvalidEndpoint(
        msg.endpoint().to_string(),
      )))
      .boxed();
    }

    self.subscribed_endpoints.insert(msg.endpoint());
    future::ready(Ok(())).boxed()
  }

  fn unsubscribe(
    &self,
    msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    if !self.endpoints.as_ref().contains(&msg.endpoint()) {
      return future::ready(Err(ButtplugDeviceError::InvalidEndpoint(
        msg.endpoint().to_string(),
      )))
      .boxed();
    }

    self.subscribed_endpoints.remove(&msg.endpoint());
    future::ready(Ok(())).boxed()
  }
}

/// Connector that wraps a SimulatedDevice for integration with the device discovery pipeline
pub struct SimulatedDeviceConnector {
  specifier: ProtocolCommunicationSpecifier,
  name: String,
  address: String,
  device: Option<SimulatedDevice>,
  endpoints: Vec<Endpoint>,
}

impl SimulatedDeviceConnector {
  /// Create a new connector
  pub fn new(
    specifier: ProtocolCommunicationSpecifier,
    name: &str,
    address: &str,
    device: SimulatedDevice,
  ) -> Self {
    let endpoints = (*device.endpoints).clone();
    Self {
      specifier,
      name: name.to_owned(),
      address: address.to_owned(),
      device: Some(device),
      endpoints,
    }
  }
}

impl Debug for SimulatedDeviceConnector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("SimulatedDeviceConnector")
      .field("specifier", &self.specifier)
      .field("name", &self.name)
      .field("address", &self.address)
      .finish()
  }
}

#[async_trait]
impl HardwareConnector for SimulatedDeviceConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    self.specifier.clone()
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    let device = self.device.take().ok_or_else(|| {
      ButtplugDeviceError::DeviceCommunicationError("Device already connected".to_string())
    })?;

    let hardware = Hardware::new(
      &self.name,
      &self.address,
      &self.endpoints,
      &None,
      false,
      Box::new(device),
    );

    Ok(Box::new(GenericHardwareSpecializer::new(hardware)))
  }
}
