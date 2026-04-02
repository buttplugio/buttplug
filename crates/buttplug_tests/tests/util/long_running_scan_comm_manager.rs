// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! A test comm manager that mimics btleplug's scanning behavior:
//! - Emits DeviceFound events when scanning starts
//! - Never emits ScanningFinished (BLE scanning runs until explicitly stopped)
//! - Re-emits devices on each scan cycle (simulating BLE re-advertisement)
//! - Tracks start/stop call counts for test assertions

use super::test_device_manager::{
  TestDeviceIdentifier,
  test_device::{TestDeviceChannelDevice, TestDeviceChannelHost, new_device_channel},
};
use buttplug_core::ButtplugResultFuture;
use buttplug_server::device::hardware::communication::{
  HardwareCommunicationManager,
  HardwareCommunicationManagerBuilder,
  HardwareCommunicationManagerEvent,
};
use buttplug_server_device_config::{BluetoothLESpecifier, ProtocolCommunicationSpecifier};
use futures::FutureExt;
use log::error;
use std::{
  collections::HashMap,
  sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, Ordering},
  },
};
use tokio::sync::mpsc::Sender;

use super::test_device_manager::test_device::{TestDevice, TestHardwareConnector};

/// Shared state between the test and the comm manager for observing behavior.
#[derive(Clone)]
pub struct LongRunningScanState {
  pub start_count: Arc<AtomicU32>,
  pub stop_count: Arc<AtomicU32>,
  pub is_scanning: Arc<AtomicBool>,
}

impl Default for LongRunningScanState {
  fn default() -> Self {
    Self {
      start_count: Arc::new(AtomicU32::new(0)),
      stop_count: Arc::new(AtomicU32::new(0)),
      is_scanning: Arc::new(AtomicBool::new(false)),
    }
  }
}

pub struct LongRunningScanCommunicationManagerBuilder {
  devices: Vec<(TestDeviceIdentifier, TestDeviceChannelDevice)>,
  state: LongRunningScanState,
}

impl LongRunningScanCommunicationManagerBuilder {
  pub fn new(state: LongRunningScanState) -> Self {
    Self {
      devices: vec![],
      state,
    }
  }

  pub fn add_test_device(&mut self, device: &TestDeviceIdentifier) -> TestDeviceChannelHost {
    let (host_channel, device_channel) = new_device_channel();
    self.devices.push((device.clone(), device_channel));
    host_channel
  }
}

impl HardwareCommunicationManagerBuilder for LongRunningScanCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(LongRunningScanCommunicationManager {
      device_sender: sender,
      devices: std::mem::take(&mut self.devices),
      state: self.state.clone(),
    })
  }
}

pub struct LongRunningScanCommunicationManager {
  device_sender: Sender<HardwareCommunicationManagerEvent>,
  devices: Vec<(TestDeviceIdentifier, TestDeviceChannelDevice)>,
  state: LongRunningScanState,
}

impl HardwareCommunicationManager for LongRunningScanCommunicationManager {
  fn name(&self) -> &'static str {
    "LongRunningScanCommunicationManager"
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    self.state.start_count.fetch_add(1, Ordering::Relaxed);
    self.state.is_scanning.store(true, Ordering::Relaxed);

    // Build DeviceFound events for all devices. Unlike TestDeviceCommunicationManager,
    // we don't consume devices — we keep them so they can be re-emitted on rescan.
    // However, HardwareConnector requires ownership of TestDevice, so we create new
    // channels for each scan cycle. The first scan's channels are the "real" ones
    // passed back to the test; subsequent scans create throwaway channels since the
    // server will reject duplicate addresses anyway.
    let mut events = vec![];
    for (device, _) in &self.devices {
      let (_, device_channel) = new_device_channel();
      let specifier = ProtocolCommunicationSpecifier::BluetoothLE(
        BluetoothLESpecifier::new_from_device(device.name(), &HashMap::new(), &[]),
      );
      let hardware = TestDevice::new(device.name(), device.address(), device_channel);
      let connector = TestHardwareConnector::new(specifier, hardware);

      events.push(HardwareCommunicationManagerEvent::DeviceFound {
        name: device.name().to_owned(),
        address: device.address().to_owned(),
        creator: Box::new(connector),
      });
    }

    let device_sender = self.device_sender.clone();
    async move {
      for event in events {
        if device_sender.send(event).await.is_err() {
          error!("Device channel no longer open.");
        }
      }
      // Deliberately do NOT send ScanningFinished — this mimics btleplug behavior
      Ok(())
    }
    .boxed()
  }

  fn stop_scanning(&mut self) -> ButtplugResultFuture {
    self.state.stop_count.fetch_add(1, Ordering::Relaxed);
    self.state.is_scanning.store(false, Ordering::Relaxed);
    // Deliberately do NOT send ScanningFinished — btleplug never does
    async { Ok(()) }.boxed()
  }

  fn scanning_status(&self) -> bool {
    self.state.is_scanning.load(Ordering::Relaxed)
  }

  fn can_scan(&self) -> bool {
    true
  }
}
