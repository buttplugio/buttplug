// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{
  test_device::{
    new_device_channel,
    TestDeviceChannelDevice,
    TestDeviceChannelHost,
    TestHardwareConnector,
  },
  TestDevice,
};
use buttplug::{
  core::ButtplugResultFuture,
  server::device::configuration::{BluetoothLESpecifier, ProtocolCommunicationSpecifier},
  server::device::hardware::communication::{
    HardwareCommunicationManager,
    HardwareCommunicationManagerBuilder,
    HardwareCommunicationManagerEvent,
  },
};
use futures::future::{self, FutureExt};
use serde::{Deserialize, Serialize};
use std::{
  collections::HashMap,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc::Sender;
use tracing::*;

pub fn generate_address() -> String {
  info!("Generating random address for test device");
  // Vaguely, not really random number. Works well enough to be an address that
  // doesn't collide.
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("Test")
    .subsec_nanos()
    .to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestDeviceIdentifier {
  name: String,
  #[serde(default = "generate_address")]
  address: String,
}

impl TestDeviceIdentifier {
  pub fn new(name: &str, address: Option<String>) -> Self {
    // Vaguely, not really random number. Works well enough to be an address that
    // doesn't collide.
    let address = address.unwrap_or_else(generate_address);
    Self {
      name: name.to_owned(),
      address,
    }
  }
}

pub struct TestDeviceCommunicationManagerBuilder {
  devices: Option<Vec<(TestDeviceIdentifier, TestDeviceChannelDevice)>>,
}

impl Default for TestDeviceCommunicationManagerBuilder {
  fn default() -> Self {
    Self {
      devices: Some(vec![]),
    }
  }
}

impl TestDeviceCommunicationManagerBuilder {
  pub fn add_test_device(&mut self, device: &TestDeviceIdentifier) -> TestDeviceChannelHost {
    let (host_channel, device_channel) = new_device_channel();
    self
      .devices
      .as_mut()
      .expect("Devices vec does not exist, is this running twice?")
      .push((device.clone(), device_channel));
    host_channel
  }
}

impl HardwareCommunicationManagerBuilder for TestDeviceCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(TestDeviceCommunicationManager::new(
      sender,
      self
        .devices
        .take()
        .expect("Devices vec does not exist, is this running twice?"),
    ))
  }
}

fn new_uninitialized_ble_test_device(
  identifier: &TestDeviceIdentifier,
  device_channel: TestDeviceChannelDevice,
) -> TestHardwareConnector {
  let address = identifier.address.clone();
  let specifier = ProtocolCommunicationSpecifier::BluetoothLE(
    BluetoothLESpecifier::new_from_device(&identifier.name, &HashMap::new(), &[]),
  );
  let hardware = TestDevice::new(&identifier.name, &address, device_channel);
  TestHardwareConnector::new(specifier, hardware)
}

pub struct TestDeviceCommunicationManager {
  device_sender: Sender<HardwareCommunicationManagerEvent>,
  devices: Vec<(TestDeviceIdentifier, TestDeviceChannelDevice)>,
  is_scanning: Arc<AtomicBool>,
}

impl TestDeviceCommunicationManager {
  pub fn new(
    device_sender: Sender<HardwareCommunicationManagerEvent>,
    devices: Vec<(TestDeviceIdentifier, TestDeviceChannelDevice)>,
  ) -> Self {
    Self {
      device_sender,
      devices,
      is_scanning: Arc::new(AtomicBool::new(false)),
    }
  }
}

impl HardwareCommunicationManager for TestDeviceCommunicationManager {
  fn name(&self) -> &'static str {
    "TestDeviceCommunicationManager"
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    if self.devices.is_empty() {
      warn!("No devices for test device comm manager to emit, did you mean to do this?");
    }

    let mut events = vec![];

    while let Some((device, test_channel)) = self.devices.pop() {
      let device_creator = new_uninitialized_ble_test_device(&device, test_channel);

      events.push(HardwareCommunicationManagerEvent::DeviceFound {
        name: device.name.clone(),
        address: device.address,
        creator: Box::new(device_creator),
      });
    }
    let device_sender = self.device_sender.clone();
    let is_scanning = self.is_scanning.clone();
    async move {
      is_scanning.store(true, Ordering::SeqCst);
      for event in events {
        if device_sender.send(event).await.is_err() {
          error!("Device channel no longer open.");
        }
      }
      // TODO Should should use
      is_scanning.store(false, Ordering::SeqCst);
      if device_sender
        .send(HardwareCommunicationManagerEvent::ScanningFinished)
        .await
        .is_err()
      {
        error!("Error sending scanning finished. Scanning may not register as finished now!");
      }
      Ok(())
    }
    .boxed()
  }

  fn stop_scanning(&mut self) -> ButtplugResultFuture {
    future::ready(Ok(())).boxed()
  }

  // Assume tests can scan for now, this would be a good place to instrument for device manager
  // testing later.
  fn can_scan(&self) -> bool {
    true
  }

  fn scanning_status(&self) -> bool {
    self.is_scanning.load(Ordering::SeqCst)
  }
}
