// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::simulated_device::{SimulatedDevice, SimulatedDeviceConnector};
use buttplug_core::ButtplugResultFuture;
use buttplug_server::device::hardware::communication::{
  HardwareCommunicationManager, HardwareCommunicationManagerBuilder,
  HardwareCommunicationManagerEvent,
};
use buttplug_server::device::hardware::{HardwareEvent, HardwareReading, HardwareWriteCmd};
use buttplug_server_device_config::{Endpoint, ProtocolCommunicationSpecifier};
use futures::future::FutureExt;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::Sender;

/// A specification for a conformance test device
pub struct ConformanceDeviceSpec {
  pub name: String,
  pub address: String,
  pub endpoints: Vec<Endpoint>,
  pub specifier: ProtocolCommunicationSpecifier,
  pub write_log: Arc<Mutex<Vec<HardwareWriteCmd>>>,
  pub read_queue: Arc<Mutex<VecDeque<HardwareReading>>>,
  pub event_sender: tokio::sync::broadcast::Sender<HardwareEvent>,
}

/// Handle for external control of a conformance device during tests
pub struct ConformanceDeviceHandle {
  pub write_log: Arc<Mutex<Vec<HardwareWriteCmd>>>,
  pub read_queue: Arc<Mutex<VecDeque<HardwareReading>>>,
  pub event_sender: tokio::sync::broadcast::Sender<HardwareEvent>,
}

/// Builder for ConformanceDeviceCommunicationManager
pub struct ConformanceDeviceCommunicationManagerBuilder {
  devices: Vec<ConformanceDeviceSpec>,
}

impl Default for ConformanceDeviceCommunicationManagerBuilder {
  fn default() -> Self {
    Self {
      devices: Vec::new(),
    }
  }
}

impl ConformanceDeviceCommunicationManagerBuilder {
  /// Create a new builder
  pub fn new() -> Self {
    Self::default()
  }

  /// Add a device to be presented during scanning
  pub fn add_device(
    &mut self,
    name: &str,
    address: &str,
    endpoints: Vec<Endpoint>,
    specifier: ProtocolCommunicationSpecifier,
  ) -> ConformanceDeviceHandle {
    let (event_sender, _) = tokio::sync::broadcast::channel(256);
    let write_log = Arc::new(Mutex::new(Vec::new()));
    let read_queue = Arc::new(Mutex::new(VecDeque::new()));

    let spec = ConformanceDeviceSpec {
      name: name.to_owned(),
      address: address.to_owned(),
      endpoints,
      specifier,
      write_log: write_log.clone(),
      read_queue: read_queue.clone(),
      event_sender: event_sender.clone(),
    };

    self.devices.push(spec);

    ConformanceDeviceHandle {
      write_log,
      read_queue,
      event_sender,
    }
  }
}

impl HardwareCommunicationManagerBuilder for ConformanceDeviceCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    // Create device instances from specs
    let mut devices = Vec::new();
    for spec in self.devices.drain(..) {
      let device = SimulatedDevice::new_with_shared_state(
        &spec.name,
        spec.endpoints.clone(),
        spec.write_log.clone(),
        spec.read_queue.clone(),
        spec.event_sender.clone(),
      );
      devices.push((spec, device));
    }

    Box::new(ConformanceDeviceCommunicationManager {
      event_sender: sender,
      devices,
    })
  }
}

/// The conformance device communication manager
pub struct ConformanceDeviceCommunicationManager {
  event_sender: Sender<HardwareCommunicationManagerEvent>,
  devices: Vec<(ConformanceDeviceSpec, SimulatedDevice)>,
}

impl HardwareCommunicationManager for ConformanceDeviceCommunicationManager {
  fn name(&self) -> &'static str {
    "ConformanceDeviceCommunicationManager"
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    let mut events = Vec::new();

    // Create DeviceFound events for each device
    for (spec, device) in self.devices.iter_mut() {
      let connector = SimulatedDeviceConnector::new(
        spec.specifier.clone(),
        &spec.name,
        &spec.address,
        device.clone(),
      );

      events.push(HardwareCommunicationManagerEvent::DeviceFound {
        name: spec.name.clone(),
        address: spec.address.clone(),
        creator: Box::new(connector),
      });
    }

    let sender = self.event_sender.clone();

    async move {
      // Emit all device found events
      for event in events {
        if sender.send(event).await.is_err() {
          break;
        }
      }

      // Emit scanning finished
      let _ = sender
        .send(HardwareCommunicationManagerEvent::ScanningFinished)
        .await;

      Ok(())
    }
    .boxed()
  }

  fn stop_scanning(&mut self) -> ButtplugResultFuture {
    async move { Ok(()) }.boxed()
  }

  fn can_scan(&self) -> bool {
    true
  }
}
