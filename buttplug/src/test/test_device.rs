use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::RawReading,
    ButtplugResultFuture,
  },
  device::{
    configuration_manager::{DeviceSpecifier, ProtocolDefinition},
    BoundedDeviceEventBroadcaster, ButtplugDeviceEvent, ButtplugDeviceImplCreator,
    DeviceImpl, DeviceImplCommand, DeviceReadCmd, DeviceSubscribeCmd, DeviceUnsubscribeCmd,
    DeviceWriteCmd, Endpoint,
  },
};
use async_channel::{bounded, Receiver, Sender};
use async_trait::async_trait;
use futures::future::{self, BoxFuture};
use std::sync::Arc;
use dashmap::DashMap;

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
  pub sender: Sender<DeviceImplCommand>,
  pub receiver: Receiver<DeviceImplCommand>,
}

impl TestDeviceEndpointChannel {
  pub fn new(sender: Sender<DeviceImplCommand>, receiver: Receiver<DeviceImplCommand>) -> Self {
    Self {
      sender,
      receiver
    }
  }
}

pub struct TestDeviceInternal {
  name: String,
  address: String,
  endpoint_channels: Arc<DashMap<Endpoint, TestDeviceEndpointChannel>>,
  pub event_broadcaster: BoundedDeviceEventBroadcaster,
}

impl TestDeviceInternal {
  pub fn new(name: &str, address: &str) -> Self {
    Self {
      name: name.to_owned(),
      address: address.to_owned(),
      endpoint_channels: Arc::new(DashMap::new()),
      event_broadcaster: BoundedDeviceEventBroadcaster::with_cap(256)
    }
  }

  pub fn name(&self) -> String {
    self.name.clone()
  }

  pub fn address(&self) -> String {
    self.address.clone()
  }

  pub fn get_endpoint_channel(&self, endpoint: &Endpoint) -> Option<TestDeviceEndpointChannel> {
    self.endpoint_channels.get(endpoint).and_then(|el| Some(el.value().clone()))
  }

  pub async fn add_endpoint(&self, endpoint: &Endpoint) {
    if !self.endpoint_channels.contains_key(endpoint) {
      let (sender, receiver) = bounded(256);
      self.endpoint_channels.insert(*endpoint, TestDeviceEndpointChannel::new(sender, receiver));
    }
  }

  pub fn disconnect(&self) -> ButtplugResultFuture {
    let broadcaster = self.event_broadcaster.clone();
    Box::pin(async move {
      broadcaster
        .send(&ButtplugDeviceEvent::Removed)
        .await
        .unwrap();
      Ok(())
    })
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
  pub event_broadcaster: BoundedDeviceEventBroadcaster,
}

impl TestDevice {
  #[allow(dead_code)]
  pub fn new(internal_device: &TestDeviceInternal) -> Self {
    let endpoints: Vec<Endpoint> = internal_device.endpoint_channels.iter().map(|el| *el.key()).collect();
    Self {
      name: internal_device.name(),
      address: internal_device.address(),
      endpoint_channels: internal_device.endpoint_channels.clone(),
      event_broadcaster: internal_device.event_broadcaster.clone(),
      endpoints
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
    let broadcaster = self.event_broadcaster.clone();
    Box::pin(async move {
      broadcaster
        .send(&ButtplugDeviceEvent::Removed)
        .await
        .unwrap();
      Ok(())
    })
  }

  fn get_event_receiver(&self) -> BoundedDeviceEventBroadcaster {
    self.event_broadcaster.clone()
  }

  fn read_value(&self, msg: DeviceReadCmd) -> BoxFuture<'static, Result<RawReading, ButtplugError>> {
    Box::pin(future::ready(Ok(RawReading::new(0, msg.endpoint, vec![]))))
  }

  fn write_value(&self, msg: DeviceWriteCmd) -> ButtplugResultFuture {
    let channels = self.endpoint_channels.clone();
    let name = self.name.to_owned();
    Box::pin(async move {
      // Since we're only accessing a channel, we can use a read lock here.
      match channels.get(&msg.endpoint) {
        Some(device_channel) => {
          // We hold both ends, can unwrap.
          device_channel.sender.send(msg.into()).await.unwrap();
          Ok(())
        }
        None => Err(
          ButtplugDeviceError::new(&format!(
            "Endpoint {} does not exist for {}",
            msg.endpoint, name
          ))
          .into(),
        ),
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
