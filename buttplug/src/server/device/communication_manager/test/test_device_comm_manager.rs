// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::test_device::{TestDeviceImplCreator, TestDeviceInternal};
use crate::{
  core::{errors::ButtplugError, ButtplugResultFuture},
  device::{
    configuration_manager::{BluetoothLESpecifier, DeviceConfigurationManager, ProtocolCommunicationSpecifier},
    ButtplugDevice,
  },
  server::device::communication_manager::{
    DeviceCommunicationEvent,
    DeviceCommunicationManager,
    DeviceCommunicationManagerBuilder,
  },
  util::device_configuration::create_test_dcm,
};
use futures::future;
use std::{
  sync::Arc,
  time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{mpsc::Sender, Mutex};

type WaitingDeviceList = Arc<Mutex<Vec<TestDeviceImplCreator>>>;

#[allow(dead_code)]
fn new_uninitialized_ble_test_device(
  name: &str,
  address: Option<String>,
) -> (Arc<TestDeviceInternal>, TestDeviceImplCreator) {
  // Vaguely, not really random number. Works well enough to be an address that
  // doesn't collide.
  let address = address.unwrap_or_else(|| {
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("Test")
      .subsec_nanos()
      .to_string()
  });
  let specifier = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(name, &[]));
  let device_impl = Arc::new(TestDeviceInternal::new(name, &address));
  let device_impl_clone = device_impl.clone();
  let device_impl_creator = TestDeviceImplCreator::new(specifier, device_impl);
  (device_impl_clone, device_impl_creator)
}

async fn new_bluetoothle_test_device_with_cfg(
  name: &str,
  device_config_mgr: Option<Arc<DeviceConfigurationManager>>,
) -> Result<(ButtplugDevice, Arc<TestDeviceInternal>), ButtplugError> {
  let config_mgr = device_config_mgr.unwrap_or_else(|| Arc::new(create_test_dcm(false)));
  let (device_impl, device_impl_creator) = new_uninitialized_ble_test_device(name, None);
  let device_impl_clone = device_impl.clone();
  let err_str = &format!("No protocol found for device {}", name);
  let device: ButtplugDevice =
    ButtplugDevice::try_create_device(config_mgr, Box::new(device_impl_creator))
      .await
      .expect("Empty option shouldn't be possible")
      .expect(err_str);
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
    let (device, creator) = new_uninitialized_ble_test_device(name, None);
    self.devices.lock().await.push(creator);
    device
  }

  pub async fn add_ble_device_with_address(
    &self,
    name: &str,
    address: &str,
  ) -> Arc<TestDeviceInternal> {
    let (device, creator) = new_uninitialized_ble_test_device(name, Some(address.to_owned()));
    self.devices.lock().await.push(creator);
    device
  }
}

#[derive(Default)]
pub struct TestDeviceCommunicationManagerBuilder {
  sender: Option<tokio::sync::mpsc::Sender<DeviceCommunicationEvent>>,
  devices: WaitingDeviceList,
}

impl TestDeviceCommunicationManagerBuilder {
  pub fn helper(&self) -> TestDeviceCommunicationManagerHelper {
    TestDeviceCommunicationManagerHelper::new(self.devices.clone())
  }
}

impl DeviceCommunicationManagerBuilder for TestDeviceCommunicationManagerBuilder {
  fn event_sender(mut self, sender: Sender<DeviceCommunicationEvent>) -> Self {
    self.sender = Some(sender);
    self
  }

  fn finish(mut self) -> Box<dyn DeviceCommunicationManager> {
    Box::new(TestDeviceCommunicationManager::new(
      self.sender.take().expect("We always have this."),
      self.devices,
    ))
  }
}

pub struct TestDeviceCommunicationManager {
  device_sender: Sender<DeviceCommunicationEvent>,
  devices: WaitingDeviceList,
}

impl TestDeviceCommunicationManager {
  pub fn new(device_sender: Sender<DeviceCommunicationEvent>, devices: WaitingDeviceList) -> Self {
    Self {
      device_sender,
      devices,
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
          .send(DeviceCommunicationEvent::DeviceFound {
            name: d
              .device()
              .as_ref()
              .map_or("Test device".to_owned(), |x| x.name()),
            address: d
              .device()
              .as_ref()
              .map_or("Test device address".to_owned(), |x| x.address()),
            creator: Box::new(d),
          })
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

  // Assume tests can scan for now, this would be a good place to instrument for device manager
  // testing later.
  fn can_scan(&self) -> bool {
    true
  }
}

#[cfg(test)]
mod test {
  use crate::{
    core::messages::{self, ButtplugMessageSpecVersion, ButtplugServerMessage},
    server::device::communication_manager::test::TestDeviceCommunicationManagerBuilder,
    server::ButtplugServer,
    util::async_manager,
  };
  use futures::StreamExt;

  #[test]
  fn test_test_device_comm_manager() {
    async_manager::block_on(async {
      let server = ButtplugServer::default();
      let recv = server.event_stream();
      pin_mut!(recv);
      let builder = TestDeviceCommunicationManagerBuilder::default();
      let helper = builder.helper();
      server
        .device_manager()
        .add_comm_manager(builder)
        .expect("Test");
      let device = helper.add_ble_device("Massage Demo").await;
      let msg =
        messages::RequestServerInfo::new("Test Client", ButtplugMessageSpecVersion::Version2);
      let mut reply = server.parse_message(msg.into()).await;
      assert!(reply.is_ok(), "Should get back ok: {:?}", reply);
      reply = server
        .parse_message(messages::StartScanning::default().into())
        .await;
      assert!(reply.is_ok(), "Should get back ok: {:?}", reply);
      // Check that we got an event back about a new device.
      let mut device_index = 0;
      info!("Waiting on device");
      while let Some(msg) = recv.next().await {
        if let ButtplugServerMessage::DeviceAdded(da) = msg {
          assert_eq!(da.device_name(), "Aneros Vivi");
          device_index = da.device_index();
          break;
        }
      }
      device.disconnect().await.expect("Test");
      info!("waiting on removed device");
      // Check that we got an event back about a removed device.
      while let Some(msg) = recv.next().await {
        match msg {
          ButtplugServerMessage::DeviceRemoved(da) => {
            assert_eq!(da.device_index(), device_index);
            return;
          }
          ButtplugServerMessage::ScanningFinished(_) => continue,
          _ => panic!(
            "Returned message was not a DeviceRemoved message or timed out: {:?}",
            msg
          ),
        }
      }
      panic!("Shouldn't get here!");
    });
  }
}
