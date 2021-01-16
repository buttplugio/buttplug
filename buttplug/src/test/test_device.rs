use crate::{core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::RawReading,
    ButtplugResultFuture,
  }, device::{
    configuration_manager::{DeviceSpecifier, ProtocolDefinition},
    ButtplugDeviceEvent,
    ButtplugDeviceImplCreator,
    DeviceImpl,
    DeviceImplCommand,
    DeviceReadCmd,
    DeviceSubscribeCmd,
    DeviceUnsubscribeCmd,
    DeviceWriteCmd,
    Endpoint,
  }, util::stream::convert_broadcast_receiver_to_stream};
use tokio::sync::{broadcast, mpsc};
use async_trait::async_trait;
use dashmap::DashMap;
use futures::{Stream, future::{self, BoxFuture}};
use std::{
  sync::Arc,
  fmt::{self, Debug}
};

pub struct TestDeviceImplCreator {
  specifier: DeviceSpecifier,
  device_impl: Option<Arc<TestDeviceInternal>>,
}

impl TestDeviceImplCreator {
  #[allow(dead_code)]
  pub fn new(specifier: DeviceSpecifier, device_impl: Arc<TestDeviceInternal>) -> Self {
    Self {
      specifier,
      device_impl: Some(device_impl),
    }
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
  fn get_specifier(&self) -> DeviceSpecifier {
    self.specifier.clone()
  }

  async fn try_create_device_impl(
    &mut self,
    protocol: ProtocolDefinition,
  ) -> Result<Box<dyn DeviceImpl>, ButtplugError> {
    let device = self.device_impl.take().unwrap();
    if let Some(btle) = &protocol.btle {
      for endpoint_map in btle.services.values() {
        for endpoint in endpoint_map.keys() {
          device.add_endpoint(endpoint).await;
        }
      }
    }
    Ok(Box::new(TestDevice::new(&device)))
  }
}

#[derive(Clone)]
pub struct TestDeviceEndpointChannel {
  pub sender: Arc<mpsc::Sender<DeviceImplCommand>>,
  // This is a sync mutex because tests should run procedurally and not conflict
  pub receiver: Arc<std::sync::Mutex<mpsc::Receiver<DeviceImplCommand>>>,
}

impl TestDeviceEndpointChannel {
  pub fn new(sender: mpsc::Sender<DeviceImplCommand>, receiver: mpsc::Receiver<DeviceImplCommand>) -> Self {
    Self { sender: Arc::new(sender), receiver: Arc::new(std::sync::Mutex::new(receiver)) }
  }
}

pub struct TestDeviceInternal {
  name: String,
  address: String,
  endpoint_channels: Arc<DashMap<Endpoint, TestDeviceEndpointChannel>>,
  pub event_sender: broadcast::Sender<ButtplugDeviceEvent>
}

impl TestDeviceInternal {
  pub fn new(name: &str, address: &str) -> Self {
    let (event_sender, _) = broadcast::channel(256);
    Self {
      name: name.to_owned(),
      address: address.to_owned(),
      endpoint_channels: Arc::new(DashMap::new()),
      event_sender
    }
  }

  pub fn name(&self) -> String {
    self.name.clone()
  }

  pub fn address(&self) -> String {
    self.address.clone()
  }

  pub fn get_endpoint_receiver(&self, endpoint: &Endpoint) -> Option<Arc<std::sync::Mutex<mpsc::Receiver<DeviceImplCommand>>>> {
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
    self
      .event_sender
      .send(ButtplugDeviceEvent::Removed)
      .unwrap();
    Box::pin(future::ready(Ok(())))
  }
}

#[derive(Clone)]
pub struct TestDevice {
  name: String,
  endpoints: Vec<Endpoint>,
  address: String,
  // This shouldn't need to be Arc<Mutex<T>>, as the channels are clonable.
  // However, it means we can only store off the device after we send it off
  // for creation in ButtplugDevice, so initialization and cloning order
  // matters here.
  pub endpoint_channels: Arc<DashMap<Endpoint, TestDeviceEndpointChannel>>,
  pub event_sender: broadcast::Sender<ButtplugDeviceEvent>
}

impl TestDevice {
  #[allow(dead_code)]
  pub fn new(internal_device: &TestDeviceInternal) -> Self {
    let endpoints: Vec<Endpoint> = internal_device
      .endpoint_channels
      .iter()
      .map(|el| *el.key())
      .collect();
    Self {
      name: internal_device.name(),
      address: internal_device.address(),
      endpoint_channels: internal_device.endpoint_channels.clone(),
      event_sender: internal_device.event_sender.clone(),
      endpoints,
    }
  }
}

impl DeviceImpl for TestDevice {
  fn name(&self) -> &str {
    &self.name
  }

  fn address(&self) -> &str {
    &self.address
  }

  fn connected(&self) -> bool {
    true
  }

  fn endpoints(&self) -> Vec<Endpoint> {
    self.endpoints.clone()
  }

  fn disconnect(&self) -> ButtplugResultFuture {
    self
      .event_sender
      .send(ButtplugDeviceEvent::Removed)
      .unwrap();
    Box::pin(future::ready(Ok(())))
  }

  fn event_stream(&self) -> Box<dyn Stream<Item = ButtplugDeviceEvent> + Unpin + Send> {
    Box::new(Box::pin(convert_broadcast_receiver_to_stream(self.event_sender.subscribe())))
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
          device_channel.sender.send(msg.into()).await.unwrap();
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
