use super::{TestDeviceImplCreator, TestDeviceInternal};
use crate::{
  core::{errors::ButtplugError, ButtplugResultFuture},
  device::{
    configuration_manager::{BluetoothLESpecifier, DeviceSpecifier, DeviceConfigurationManager},
    ButtplugDevice,
  },
  server::comm_managers::{
    DeviceCommunicationEvent,
    DeviceCommunicationManager,
    DeviceCommunicationManagerCreator,
  },
};
use async_channel::Sender;
use async_mutex::Mutex;
use futures::future;
use std::{
  sync::Arc,
  time::{SystemTime, UNIX_EPOCH},
};

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

pub async fn new_bluetoothle_test_device_with_cfg(
  name: &str,
  device_config_mgr: Option<Arc<DeviceConfigurationManager>>
) -> Result<(ButtplugDevice, Arc<TestDeviceInternal>), ButtplugError> {
  let config_mgr = device_config_mgr.unwrap_or(Arc::new(DeviceConfigurationManager::default()));
  let (device_impl, device_impl_creator) = new_uninitialized_ble_test_device(name);
  let device_impl_clone = device_impl.clone();
  let device: ButtplugDevice = ButtplugDevice::try_create_device(config_mgr, Box::new(device_impl_creator))
    .await
    .unwrap()
    .unwrap();
  Ok((device, device_impl_clone))
}

pub async fn new_bluetoothle_test_device(
  name: &str,
) -> Result<(ButtplugDevice, Arc<TestDeviceInternal>), ButtplugError> {
  new_bluetoothle_test_device_with_cfg(name, None).await
}

pub struct TestDeviceCommunicationManagerHelper {
  devices: WaitingDeviceList,
}

impl TestDeviceCommunicationManagerHelper {
  pub(super) fn new(device_list: WaitingDeviceList) -> Self {
    Self {
      devices: device_list,
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
  fn name(&self) -> &'static str {
    "TestDeviceCommunicationManager"
  }

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
          .is_err()
        {
          error!("Device channel no longer open.");
        }
      }
      if device_sender
        .send(DeviceCommunicationEvent::ScanningFinished)
        .await
        .is_err()
      {
        error!("Error sending scanning finished. Scanning may not register as finished now!");
      }
      Ok(())
    })
  }

  fn stop_scanning(&self) -> ButtplugResultFuture {
    Box::pin(future::ready(Ok(())))
  }
}

#[cfg(test)]
mod test {
  use crate::{
    core::messages::{self, ButtplugMessageSpecVersion, ButtplugServerMessage},
    server::ButtplugServer,
    util::async_manager,
  };
  use futures::StreamExt;

  #[test]
  fn test_test_device_comm_manager() {
    let (server, mut recv) = ButtplugServer::default();
    async_manager::block_on(async {
      let helper = server.add_test_comm_manager().unwrap();
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
      while let Some(msg) = recv.next().await {
        if let ButtplugServerMessage::DeviceAdded(da) = msg {
          assert_eq!(da.device_name, "Aneros Vivi");
          break;
        }
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
