use super::{TestDeviceImplCreator, TestDeviceInternal};
use crate::{
  core::{ButtplugResultFuture, errors::ButtplugError},
  server::comm_managers::{
    DeviceCommunicationEvent, DeviceCommunicationManager, DeviceCommunicationManagerCreator,
  },
  device::{
    configuration_manager::{BluetoothLESpecifier, DeviceSpecifier},
    ButtplugDevice,
  },
};
use std::{sync::Arc, time::{SystemTime, UNIX_EPOCH}};
use async_mutex::Mutex;
use async_channel::Sender;
use futures::future;

type WaitingDeviceList = Arc<Mutex<Vec<TestDeviceImplCreator>>>;

#[allow(dead_code)]
pub fn new_uninitialized_ble_test_device(
  name: &str,
) -> (Arc<TestDeviceInternal>, TestDeviceImplCreator) {
  // Vaguely, not really random number. Works well enough to be an address that
  // doesn't collide.
  let nanos = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .subsec_nanos();
  let specifier = DeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(name));
  let device_impl = Arc::new(TestDeviceInternal::new(name, &nanos.to_string()));
  let device_impl_clone = device_impl.clone();
  let device_impl_creator = TestDeviceImplCreator::new(specifier, device_impl);
  (device_impl_clone, device_impl_creator)
}

pub async fn new_bluetoothle_test_device(
  name: &str,
) -> Result<(ButtplugDevice, Arc<TestDeviceInternal>), ButtplugError> {
  let (device_impl, device_impl_creator) =
    new_uninitialized_ble_test_device(name);
  let device_impl_clone = device_impl.clone();
  let device: ButtplugDevice = ButtplugDevice::try_create_device(Box::new(device_impl_creator))
    .await
    .unwrap()
    .unwrap();
  Ok((device, device_impl_clone))
}

pub struct TestDeviceCommunicationManagerHelper {
  devices: WaitingDeviceList,
}

impl TestDeviceCommunicationManagerHelper {
  pub(super) fn new(device_list: WaitingDeviceList) -> Self {
    Self {
      devices: device_list
    }
  }

  pub async fn add_ble_device(&self, name: &str) -> Arc<TestDeviceInternal> {
    let (device, creator) = new_uninitialized_ble_test_device(name);
    self.devices.lock().await.push(creator);
    device
  }
}

pub struct TestDeviceCommunicationManager {
  device_sender: Sender<DeviceCommunicationEvent>,
  devices: WaitingDeviceList,
}

impl TestDeviceCommunicationManager {
  pub fn helper(&self) -> TestDeviceCommunicationManagerHelper {
    TestDeviceCommunicationManagerHelper::new(self.devices.clone())
  }
}

impl DeviceCommunicationManagerCreator for TestDeviceCommunicationManager {
  fn new(device_sender: Sender<DeviceCommunicationEvent>) -> Self {
    Self {
      device_sender,
      devices: Arc::new(Mutex::new(vec![])),
    }
  }
}

impl DeviceCommunicationManager for TestDeviceCommunicationManager {
  fn start_scanning(&self) -> ButtplugResultFuture {
    let devices_vec = self.devices.clone();
    let device_sender = self.device_sender.clone();
    Box::pin(async move {
      let mut devices = devices_vec.lock().await;
      if devices.is_empty() {
        panic!("No devices for test device comm manager to emit!");
      }
      while let Some(d) = devices.pop() {
        if device_sender
          .send(DeviceCommunicationEvent::DeviceFound(Box::new(d)))
          .await
          .is_err() {
            error!("Device channel no longer open.");
          }
      }
      Ok(())
    })
  }

  fn stop_scanning(&self) -> ButtplugResultFuture {
    Box::pin(future::ready(Ok(())))
  }

  fn is_scanning(&self) -> bool {
    false
  }
}

#[cfg(test)]
mod test {
  use crate::{
    core::messages::{self, ButtplugMessageSpecVersion, ButtplugServerMessage},
    server::ButtplugServer,
    util::async_manager
  };
  use futures::StreamExt;

  #[test]
  fn test_test_device_comm_manager() {
    let (mut server, mut recv) = ButtplugServer::new("Test Server", 0);
    async_manager::block_on(async {
      let helper = server.add_test_comm_manager();
      let device = helper.add_ble_device("Massage Demo").await;
      let msg =
        messages::RequestServerInfo::new("Test Client", ButtplugMessageSpecVersion::Version2);
      let mut reply = server.parse_message(msg.into()).await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      reply = server
        .parse_message(messages::StartScanning::default().into())
        .await;
      assert!(reply.is_ok(), format!("Should get back ok: {:?}", reply));
      // Check that we got an event back about a new device.
      let msg = recv.next().await.unwrap();
      if let ButtplugServerMessage::DeviceAdded(da) = msg {
        assert_eq!(da.device_name, "Aneros Vivi");
      } else {
        panic!(format!(
          "Returned message was not a DeviceAdded message or timed out: {:?}",
          msg
        ));
      }
      device.disconnect().await.unwrap();
      // Check that we got an event back about a removed device.
      let msg = recv.next().await.unwrap();
      if let ButtplugServerMessage::DeviceRemoved(da) = msg {
        assert_eq!(da.device_index, 0);
      } else {
        panic!(format!(
          "Returned message was not a DeviceRemoved message or timed out: {:?}",
          msg
        ));
      }
    });
  }
}
