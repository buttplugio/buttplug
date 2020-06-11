use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::RawReading,
    ButtplugResultFuture,
  },
  device::{
    configuration_manager::{BluetoothLESpecifier, DeviceSpecifier, ProtocolDefinition},
    BoundedDeviceEventBroadcaster, ButtplugDevice, ButtplugDeviceEvent, ButtplugDeviceImplCreator,
    DeviceImpl, DeviceImplCommand, DeviceReadCmd, DeviceSubscribeCmd, DeviceUnsubscribeCmd,
    DeviceWriteCmd, Endpoint,
  },
};
use async_mutex::Mutex;
use async_channel::{bounded, Receiver, Sender};
use async_trait::async_trait;
use futures::future::{self, BoxFuture};
use std::{
  collections::HashMap,
  sync::Arc,
};

pub struct TestDeviceImplCreator {
  specifier: DeviceSpecifier,
  device_impl: Option<TestDevice>,
}

impl TestDeviceImplCreator {
  #[allow(dead_code)]
  pub fn new(specifier: DeviceSpecifier, device_impl: TestDevice) -> Self {
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
    let mut device = self.device_impl.take().unwrap();
    if let Some(btle) = &protocol.btle {
      for endpoint_map in btle.services.values() {
        for endpoint in endpoint_map.keys() {
          device.add_endpoint(*endpoint).await;
        }
      }
    }
    Ok(Box::new(device))
  }
}

type EndpointChannels =
  Arc<Mutex<HashMap<Endpoint, (Sender<DeviceImplCommand>, Receiver<DeviceImplCommand>)>>>;

#[derive(Clone)]
pub struct TestDevice {
  name: String,
  endpoints: Vec<Endpoint>,
  address: String,
  // This shouldn't need to be Arc<Mutex<T>>, as the channels are clonable.
  // However, it means we can only store off the device after we send it off
  // for creation in ButtplugDevice, so initialization and cloning order
  // matters here.
  pub endpoint_channels: EndpointChannels,
  pub event_broadcaster: BoundedDeviceEventBroadcaster,
}

impl TestDevice {
  #[allow(dead_code)]
  pub fn new(name: &str, endpoints: Vec<Endpoint>) -> Self {
    let mut endpoint_channels = HashMap::new();
    for endpoint in &endpoints {
      let (sender, receiver) = bounded(256);
      endpoint_channels.insert(*endpoint, (sender, receiver));
    }
    let event_broadcaster = BoundedDeviceEventBroadcaster::with_cap(256);
    Self {
      name: name.to_string(),
      address: "".to_string(),
      endpoints,
      endpoint_channels: Arc::new(Mutex::new(endpoint_channels)),
      event_broadcaster,
    }
  }

  pub async fn add_endpoint(&mut self, endpoint: Endpoint) {
    let mut endpoint_channels = self.endpoint_channels.lock().await;
    if !endpoint_channels.contains_key(&endpoint) {
      let (sender, receiver) = bounded(256);
      endpoint_channels.insert(endpoint, (sender, receiver));
    }
  }

  #[allow(dead_code)]
  pub fn new_bluetoothle_test_device_impl_creator(
    name: &str,
  ) -> (TestDevice, TestDeviceImplCreator) {
    let specifier = DeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(name));
    let device_impl = TestDevice::new(name, vec![]);
    let device_impl_clone = device_impl.clone();
    let device_impl_creator = TestDeviceImplCreator::new(specifier, device_impl);
    (device_impl_clone, device_impl_creator)
  }

  #[allow(dead_code)]
  pub async fn new_bluetoothle_test_device(
    name: &str,
  ) -> Result<(ButtplugDevice, TestDevice), ButtplugError> {
    let (device_impl, device_impl_creator) =
      TestDevice::new_bluetoothle_test_device_impl_creator(name);
    let device_impl_clone = device_impl.clone();
    let device: ButtplugDevice = ButtplugDevice::try_create_device(Box::new(device_impl_creator))
      .await
      .unwrap()
      .unwrap();
    Ok((device, device_impl_clone))
  }

  #[allow(dead_code)]
  pub async fn get_endpoint_channel_clone(
    &self,
    endpoint: Endpoint,
  ) -> (Sender<DeviceImplCommand>, Receiver<DeviceImplCommand>) {
    let endpoint_channels = self.endpoint_channels.lock().await;
    let (sender, receiver) = endpoint_channels.get(&endpoint).unwrap();
    (sender.clone(), receiver.clone())
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
      match channels.lock().await.get(&msg.endpoint) {
        Some((sender, _)) => {
          sender.send(msg.into()).await;
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
