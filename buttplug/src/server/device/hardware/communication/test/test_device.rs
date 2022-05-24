// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::{Endpoint, RawReading},
    ButtplugResultFuture,
  },
  server::device::{
    configuration::{ProtocolCommunicationSpecifier},
    hardware::{
      HardwareConnector,
      HardwareSpecializer,
    HardwareEvent,
    Hardware,
    HardwareCommand,
    HardwareInternal,
    HardwareReadCmd,
    HardwareSubscribeCmd,
    HardwareUnsubscribeCmd,
    HardwareWriteCmd,
    },
  },
};
use async_trait::async_trait;
use dashmap::DashMap;
use futures::future::{self, BoxFuture};
use std::{
  fmt::{self, Debug},
  sync::Arc,
};
use tokio::sync::{broadcast, mpsc};

pub struct TestHardwareConnector {
  specifier: ProtocolCommunicationSpecifier,
  hardware: Arc<TestDeviceInternal>,
}

impl TestHardwareConnector {
  #[allow(dead_code)]
  pub fn new(specifier: ProtocolCommunicationSpecifier, hardware: Arc<TestDeviceInternal>) -> Self {
    Self {
      specifier,
      hardware,
    }
  }

  pub fn device(&self) -> Arc<TestDeviceInternal> {
    self.hardware.clone()
  }
}

impl Debug for TestHardwareConnector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("TestHardwareCreator")
      .field("specifier", &self.specifier)
      .finish()
  }
}

#[async_trait]
impl HardwareConnector for TestHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    self.specifier.clone()
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    Ok(Box::new(TestHardwareSpecializer::new(self.hardware.clone())))
  }
}

pub struct TestHardwareSpecializer {
  hardware: Arc<TestDeviceInternal>,
}

impl TestHardwareSpecializer {
  fn new(hardware: Arc<TestDeviceInternal>) -> Self {
    Self {
      hardware
    }
  }
}

#[async_trait]
impl HardwareSpecializer for TestHardwareSpecializer {
  async fn specialize(
    &mut self,
    specifiers: &Vec<ProtocolCommunicationSpecifier>,
  ) -> Result<Hardware, ButtplugDeviceError> {
    let device = self.hardware.clone();
    if let Some(ProtocolCommunicationSpecifier::BluetoothLE(btle)) = specifiers.iter().find(|x| matches!(x, ProtocolCommunicationSpecifier::BluetoothLE(_))) {
      for endpoint_map in btle.services().values() {
        for endpoint in endpoint_map.keys() {
          device.add_endpoint(endpoint).await;
        }
      }
    }
    let endpoints: Vec<Endpoint> = device
      .endpoint_channels
      .iter()
      .map(|el| *el.key())
      .collect();
    let hardware_internal = TestDevice::new(&device);
    let hardware = Hardware::new(
      &device.name(),
      &device.address(),
      &endpoints,
      Box::new(hardware_internal),
    );
    Ok(hardware)
  }
}

#[derive(Clone)]
pub struct TestDeviceEndpointChannel {
  pub sender: Arc<mpsc::Sender<HardwareCommand>>,
  // This is a sync mutex because tests should run procedurally and not conflict
  pub receiver: Arc<std::sync::Mutex<mpsc::Receiver<HardwareCommand>>>,
}

impl TestDeviceEndpointChannel {
  pub fn new(
    sender: mpsc::Sender<HardwareCommand>,
    receiver: mpsc::Receiver<HardwareCommand>,
  ) -> Self {
    Self {
      sender: Arc::new(sender),
      receiver: Arc::new(std::sync::Mutex::new(receiver)),
    }
  }
}

pub struct TestDeviceInternal {
  name: String,
  address: String,
  endpoint_channels: Arc<DashMap<Endpoint, TestDeviceEndpointChannel>>,
  event_sender: broadcast::Sender<HardwareEvent>,
}

impl TestDeviceInternal {
  pub fn new(name: &str, address: &str) -> Self {
    let (event_sender, _) = broadcast::channel(256);
    Self {
      name: name.to_owned(),
      address: address.to_owned(),
      endpoint_channels: Arc::new(DashMap::new()),
      event_sender,
    }
  }

  pub fn sender(&self) -> broadcast::Sender<HardwareEvent> {
    self.event_sender.clone()
  }

  pub fn send_event(&self, event: HardwareEvent) {
    self.event_sender.send(event).expect("Test");
  }

  pub fn name(&self) -> String {
    self.name.clone()
  }

  pub fn address(&self) -> String {
    self.address.clone()
  }

  pub fn endpoint_receiver(
    &self,
    endpoint: &Endpoint,
  ) -> Option<Arc<std::sync::Mutex<mpsc::Receiver<HardwareCommand>>>> {
    self
      .endpoint_channels
      .get(endpoint)
      .map(|el| el.value().receiver.clone())
  }

  pub async fn add_endpoint(&self, endpoint: &Endpoint) {
    if !self.endpoint_channels.contains_key(endpoint) {
      let (sender, receiver) = mpsc::channel(256);
      self
        .endpoint_channels
        .insert(*endpoint, TestDeviceEndpointChannel::new(sender, receiver));
    }
  }

  pub fn disconnect(&self) -> ButtplugResultFuture {
    let sender = self.event_sender.clone();
    let address = self.address.clone();
    Box::pin(async move {
      sender
        .send(HardwareEvent::Disconnected(address))
        .expect("Test");
      Ok(())
    })
  }
}

pub struct TestDevice {
  address: String,
  // This shouldn't need to be Arc<Mutex<T>>, as the channels are clonable.
  // However, it means we can only store off the device after we send it off
  // for creation in ButtplugDevice, so initialization and cloning order
  // matters here.
  pub endpoint_channels: Arc<DashMap<Endpoint, TestDeviceEndpointChannel>>,
  event_sender: broadcast::Sender<HardwareEvent>,
}

impl TestDevice {
  #[allow(dead_code)]
  pub fn new(internal_device: &TestDeviceInternal) -> Self {
    Self {
      address: internal_device.address(),
      endpoint_channels: internal_device.endpoint_channels.clone(),
      event_sender: internal_device.sender(),
    }
  }
}

impl HardwareInternal for TestDevice {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.event_sender.subscribe()
  }

  fn connected(&self) -> bool {
    true
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let sender = self.event_sender.clone();
    let address = self.address.clone();
    Box::pin(async move {
      sender
        .send(HardwareEvent::Disconnected(address))
        .expect("Test");
      Ok(())
    })
  }

  fn read_value(
    &self,
    msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<RawReading, ButtplugDeviceError>> {
    Box::pin(future::ready(Ok(RawReading::new(0, msg.endpoint, vec![]))))
  }

  fn write_value(&self, msg: &HardwareWriteCmd) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let channels = self.endpoint_channels.clone();
    let data_command = msg.clone().into();
    let endpoint = msg.endpoint;
    Box::pin(async move {
      // Since we're only accessing a channel, we can use a read lock here.
      match channels.get(&endpoint) {
        Some(device_channel) => {
          // We hold both ends, can unwrap.
          device_channel.sender.send(data_command).await.expect("Test");
          Ok(())
        }
        None => Err(ButtplugDeviceError::InvalidEndpoint(endpoint)),
      }
    })
  }

  fn subscribe(&self, _msg: &HardwareSubscribeCmd) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    Box::pin(future::ready(Err(ButtplugDeviceError::UnhandledCommand("Test device does not support subscribe".to_owned()))))
  }

  fn unsubscribe(&self, _msg: &HardwareUnsubscribeCmd) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    Box::pin(future::ready(Err(ButtplugDeviceError::UnhandledCommand("Test device does not support unsubscribe".to_owned()))))
  }
}
