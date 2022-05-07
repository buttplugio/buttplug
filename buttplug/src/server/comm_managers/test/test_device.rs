// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::RawReading,
    ButtplugResultFuture,
  },
  device::{
    configuration_manager::{ProtocolCommunicationSpecifier, ProtocolDeviceConfiguration},
    ButtplugDeviceEvent,
    ButtplugDeviceImplCreator,
    DeviceImpl,
    DeviceImplCommand,
    DeviceImplInternal,
    DeviceReadCmd,
    DeviceSubscribeCmd,
    DeviceUnsubscribeCmd,
    DeviceWriteCmd,
    Endpoint,
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

pub struct TestDeviceImplCreator {
  specifier: ProtocolCommunicationSpecifier,
  device_impl: Option<Arc<TestDeviceInternal>>,
}

impl TestDeviceImplCreator {
  #[allow(dead_code)]
  pub fn new(specifier: ProtocolCommunicationSpecifier, device_impl: Arc<TestDeviceInternal>) -> Self {
    Self {
      specifier,
      device_impl: Some(device_impl),
    }
  }

  pub fn device(&self) -> &Option<Arc<TestDeviceInternal>> {
    &self.device_impl
  }
}

impl Debug for TestDeviceImplCreator {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("TestDeviceImplCreator")
      .field("specifier", &self.specifier)
      .finish()
  }
}

#[async_trait]
impl ButtplugDeviceImplCreator for TestDeviceImplCreator {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    self.specifier.clone()
  }

  async fn try_create_device_impl(
    &mut self,
    protocol: ProtocolDeviceConfiguration,
  ) -> Result<DeviceImpl, ButtplugError> {
    let device = self
      .device_impl
      .take()
      .expect("We'll always have this at this point");
    if let Some(ProtocolCommunicationSpecifier::BluetoothLE(btle)) = protocol.specifiers().iter().find(|x| matches!(x, ProtocolCommunicationSpecifier::BluetoothLE(_))) {
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
    let device_impl_internal = TestDevice::new(&device);
    let device_impl = DeviceImpl::new(
      &device.name(),
      &device.address(),
      &endpoints,
      Box::new(device_impl_internal),
    );
    Ok(device_impl)
  }
}

#[derive(Clone)]
pub struct TestDeviceEndpointChannel {
  pub sender: Arc<mpsc::Sender<DeviceImplCommand>>,
  // This is a sync mutex because tests should run procedurally and not conflict
  pub receiver: Arc<std::sync::Mutex<mpsc::Receiver<DeviceImplCommand>>>,
}

impl TestDeviceEndpointChannel {
  pub fn new(
    sender: mpsc::Sender<DeviceImplCommand>,
    receiver: mpsc::Receiver<DeviceImplCommand>,
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
  event_sender: broadcast::Sender<ButtplugDeviceEvent>,
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

  pub fn sender(&self) -> broadcast::Sender<ButtplugDeviceEvent> {
    self.event_sender.clone()
  }

  pub fn send_event(&self, event: ButtplugDeviceEvent) {
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
  ) -> Option<Arc<std::sync::Mutex<mpsc::Receiver<DeviceImplCommand>>>> {
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
        .send(ButtplugDeviceEvent::Removed(address))
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
  event_sender: broadcast::Sender<ButtplugDeviceEvent>,
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

impl DeviceImplInternal for TestDevice {
  fn event_stream(&self) -> broadcast::Receiver<ButtplugDeviceEvent> {
    self.event_sender.subscribe()
  }

  fn connected(&self) -> bool {
    true
  }

  fn disconnect(&self) -> ButtplugResultFuture {
    let sender = self.event_sender.clone();
    let address = self.address.clone();
    Box::pin(async move {
      sender
        .send(ButtplugDeviceEvent::Removed(address))
        .expect("Test");
      Ok(())
    })
  }

  fn read_value(
    &self,
    msg: DeviceReadCmd,
  ) -> BoxFuture<'static, Result<RawReading, ButtplugError>> {
    Box::pin(future::ready(Ok(RawReading::new(0, msg.endpoint, vec![]))))
  }

  fn write_value(&self, msg: DeviceWriteCmd) -> ButtplugResultFuture {
    let channels = self.endpoint_channels.clone();
    Box::pin(async move {
      // Since we're only accessing a channel, we can use a read lock here.
      match channels.get(&msg.endpoint) {
        Some(device_channel) => {
          // We hold both ends, can unwrap.
          device_channel.sender.send(msg.into()).await.expect("Test");
          Ok(())
        }
        None => Err(ButtplugDeviceError::InvalidEndpoint(msg.endpoint).into()),
      }
    })
  }

  fn subscribe(&self, _msg: DeviceSubscribeCmd) -> ButtplugResultFuture {
    Box::pin(future::ready(Ok(())))
  }

  fn unsubscribe(&self, _msg: DeviceUnsubscribeCmd) -> ButtplugResultFuture {
    Box::pin(future::ready(Ok(())))
  }
}
