use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::RawReading,
  },
  device::{
    configuration_manager::{BluetoothLESpecifier, DeviceSpecifier, ProtocolDefinition},
    BoundedDeviceEventBroadcaster,
    ButtplugDevice,
    ButtplugDeviceEvent,
    ButtplugDeviceImplCreator,
    DeviceImpl,
    DeviceImplCommand,
    DeviceReadCmd,
    DeviceSubscribeCmd,
    DeviceUnsubscribeCmd,
    DeviceWriteCmd,
    Endpoint,
  },
};
use async_std::sync::{channel, Arc, Receiver, RwLock, Sender};
use async_trait::async_trait;
use std::collections::HashMap;

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
  Arc<RwLock<HashMap<Endpoint, (Sender<DeviceImplCommand>, Receiver<DeviceImplCommand>)>>>;

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
      let (sender, receiver) = channel(256);
      endpoint_channels.insert(endpoint.clone(), (sender, receiver));
    }
    let event_broadcaster = BoundedDeviceEventBroadcaster::with_cap(256);
    Self {
      name: name.to_string(),
      address: "".to_string(),
      endpoints,
      endpoint_channels: Arc::new(RwLock::new(endpoint_channels)),
      event_broadcaster,
    }
  }

  pub async fn add_endpoint(&mut self, endpoint: Endpoint) {
    let mut endpoint_channels = self.endpoint_channels.write().await;
    if !endpoint_channels.contains_key(&endpoint) {
      let (sender, receiver) = channel(256);
      endpoint_channels.insert(endpoint.clone(), (sender, receiver));
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
    let endpoint_channels = self.endpoint_channels.read().await;
    let (sender, receiver) = endpoint_channels.get(&endpoint).unwrap();
    (sender.clone(), receiver.clone())
  }
}

#[async_trait]
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

  async fn disconnect(&mut self) {
    self
      .event_broadcaster
      .send(&ButtplugDeviceEvent::Removed)
      .await
      .unwrap();
  }

  fn box_clone(&self) -> Box<dyn DeviceImpl> {
    Box::new((*self).clone())
  }

  fn get_event_receiver(&self) -> BoundedDeviceEventBroadcaster {
    self.event_broadcaster.clone()
  }

  async fn read_value(&self, msg: DeviceReadCmd) -> Result<RawReading, ButtplugError> {
    Ok(RawReading::new(0, msg.endpoint, vec![]))
  }

  async fn write_value(&self, msg: DeviceWriteCmd) -> Result<(), ButtplugError> {
    // Since we're only accessing a channel, we can use a read lock here.
    let endpoint_channels = self.endpoint_channels.read().await;
    match endpoint_channels.get(&msg.endpoint) {
      Some((sender, _)) => {
        sender.send(msg.into()).await;
        Ok(())
      }
      None => Err(
        ButtplugDeviceError::new(&format!(
          "Endpoint {} does not exist for {}",
          msg.endpoint, self.name
        ))
        .into(),
      ),
    }
  }

  async fn subscribe(&self, _msg: DeviceSubscribeCmd) -> Result<(), ButtplugError> {
    Ok(())
  }

  async fn unsubscribe(&self, _msg: DeviceUnsubscribeCmd) -> Result<(), ButtplugError> {
    Ok(())
  }
}
