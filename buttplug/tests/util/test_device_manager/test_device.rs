// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug::{
  core::{
    errors::ButtplugDeviceError,
    messages::{Endpoint, RawReading},
  },
  server::device::{
    configuration::ProtocolCommunicationSpecifier,
    hardware::{
      Hardware,
      HardwareCommand,
      HardwareConnector,
      HardwareEvent,
      HardwareInternal,
      HardwareReadCmd,
      HardwareSpecializer,
      HardwareSubscribeCmd,
      HardwareUnsubscribeCmd,
      HardwareWriteCmd,
    },
  }, util::async_manager,
};
use async_trait::async_trait;
use futures::future::{self, BoxFuture, FutureExt};
use std::{
  fmt::{self, Debug},
  collections::HashSet,
};
use tokio::sync::{broadcast, mpsc};

pub struct TestHardwareConnector {
  specifier: ProtocolCommunicationSpecifier,
  hardware: Option<TestDevice>,
}

impl TestHardwareConnector {
  #[allow(dead_code)]
  pub fn new(specifier: ProtocolCommunicationSpecifier, hardware: TestDevice) -> Self {
    Self {
      specifier,
      hardware: Some(hardware),
    }
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
    Ok(Box::new(TestHardwareSpecializer::new(
      self.hardware.take().expect("Test"),
    )))
  }
}

pub struct TestHardwareSpecializer {
  hardware: Option<TestDevice>,
}

impl TestHardwareSpecializer {
  fn new(hardware: TestDevice) -> Self {
    Self { hardware: Some(hardware) }
  }
}

#[async_trait]
impl HardwareSpecializer for TestHardwareSpecializer {
  async fn specialize(
    &mut self,
    specifiers: &[ProtocolCommunicationSpecifier],
  ) -> Result<Hardware, ButtplugDeviceError> {
    let mut device = self.hardware.take().expect("Test");
    let mut endpoints = vec![];
    if let Some(ProtocolCommunicationSpecifier::BluetoothLE(btle)) = specifiers
      .iter()
      .find(|x| matches!(x, ProtocolCommunicationSpecifier::BluetoothLE(_)))
    {
      for endpoint_map in btle.services().values() {
        for endpoint in endpoint_map.keys() {
          device.add_endpoint(endpoint);
          endpoints.push(*endpoint);
        }
      }
    }
    let hardware = Hardware::new(
      &device.name(),
      &device.address(),
      &endpoints,
      Box::new(device),
    );
    Ok(hardware)
  }
}

pub struct TestDeviceChannelHost {
  pub sender: mpsc::Sender<HardwareEvent>,
  pub receiver: mpsc::Receiver<HardwareCommand>,
}

pub struct TestDeviceChannelDevice {
  pub sender: mpsc::Sender<HardwareCommand>,
  pub receiver: mpsc::Receiver<HardwareEvent>,
}

pub fn new_device_channel(
) -> (TestDeviceChannelHost, TestDeviceChannelDevice) {
  let (host_sender, device_receiver) = mpsc::channel(256);
  let (device_sender, host_receiver) = mpsc::channel(256);
  (TestDeviceChannelHost {
    sender: host_sender,
    receiver: host_receiver,
  },
  TestDeviceChannelDevice {
    sender: device_sender,
    receiver: device_receiver  
  })
}

pub struct TestDevice {
  name: String,
  address: String,
  endpoints: HashSet<Endpoint>,
  test_device_channel: mpsc::Sender<HardwareCommand>,
  event_sender: broadcast::Sender<HardwareEvent>,
}

impl TestDevice {
  #[allow(dead_code)]
  pub fn new(name: &str, address: &str, test_device_channel: TestDeviceChannelDevice) -> Self {
    let (event_sender, _) = broadcast::channel(256);

    let event_sender_clone = event_sender.clone();
    let address_clone = address.to_owned();
    let (command_sender, mut receiver) = (test_device_channel.sender, test_device_channel.receiver);
    async_manager::spawn(async move {
      while let Some(event) = receiver.recv().await {
        if let HardwareEvent::Disconnected(_) = event {
          event_sender_clone
            .send(HardwareEvent::Disconnected(address_clone.clone()))
            .expect("Test");
        } else {
          event_sender_clone
            .send(event)
            .expect("Test");
        }
      }
    });

    Self {
      name: name.to_owned(),
      address: address.to_owned(),
      endpoints: HashSet::new(),
      test_device_channel: command_sender,
      event_sender
    }
  }

  pub fn add_endpoint(&mut self, endpoint: &Endpoint) {
    self.endpoints.insert(*endpoint);
  }

  pub fn name(&self) -> String {
    self.name.clone()
  }

  pub fn address(&self) -> String {
    self.address.clone()
  }
}

impl HardwareInternal for TestDevice {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.event_sender.subscribe()
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let sender = self.event_sender.clone();
    let address = self.address.clone();
    async move {
      sender
        .send(HardwareEvent::Disconnected(address))
        .expect("Test");
      Ok(())
    }.boxed()
  }

  fn read_value(
    &self,
    msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<RawReading, ButtplugDeviceError>> {
    future::ready(Ok(RawReading::new(0, msg.endpoint, vec![]))).boxed()
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    if !self.endpoints.contains(&msg.endpoint) {
      return future::ready(Err(ButtplugDeviceError::InvalidEndpoint(msg.endpoint))).boxed();
    }
    let sender = self.test_device_channel.clone();
    let data_command = msg.clone().into();
    async move {
      sender
        .send(data_command)
        .await
        .expect("Test");
      Ok(())
    }.boxed()
  }

  fn subscribe(
    &self,
    _msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "Test device does not support subscribe".to_owned(),
    ))).boxed()
  }

  fn unsubscribe(
    &self,
    _msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "Test device does not support unsubscribe".to_owned(),
    ))).boxed()
  }
}
