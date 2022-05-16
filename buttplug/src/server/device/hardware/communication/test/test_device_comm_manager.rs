// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::test_device::{TestHardwareCreator, TestDeviceInternal};
use crate::{
  core::{errors::ButtplugError, ButtplugResultFuture},
  server::device::{
    configuration::{BluetoothLESpecifier, DeviceConfigurationManager, ProtocolCommunicationSpecifier},
    hardware::ButtplugDevice,
  },
  server::{
    device::{
      hardware::HardwareCreator,
      hardware::communication::{
        DeviceCommunicationEvent,
        DeviceCommunicationManager,
        DeviceCommunicationManagerBuilder,
      }      
    }
  },
  util::device_configuration::create_test_dcm,
};
use futures::future;
use std::{
  sync::{Arc, atomic::{AtomicBool, Ordering}},
  time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{mpsc::Sender, Mutex};

type WaitingDeviceList = Arc<Mutex<Vec<TestHardwareCreator>>>;

#[allow(dead_code)]
fn new_uninitialized_ble_test_device(
  name: &str,
  address: Option<String>,
) -> (Arc<TestDeviceInternal>, TestHardwareCreator) {
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
  let hardware = Arc::new(TestDeviceInternal::new(name, &address));
  let hardware_clone = hardware.clone();
  let hardware_creator = TestHardwareCreator::new(specifier, hardware);
  (hardware_clone, hardware_creator)
}

async fn new_bluetoothle_test_device_with_cfg(
  name: &str,
  device_config_mgr: Option<Arc<DeviceConfigurationManager>>,
) -> Result<(ButtplugDevice, Arc<TestDeviceInternal>), ButtplugError> {
  let config_mgr = device_config_mgr.unwrap_or_else(|| Arc::new(create_test_dcm(false)));
  let (hardware, hardware_creator) = new_uninitialized_ble_test_device(name, None);
  let hardware_clone = hardware.clone();
  let err_str = &format!("No protocol found for device {}", name);
  let protocol_builder = config_mgr.protocol_instance_factory(&hardware_creator.specifier()).expect("Test code, should exist.");
  let device: ButtplugDevice =
    ButtplugDevice::try_create_device(protocol_builder, Box::new(hardware_creator))
      .await
      .expect("Empty option shouldn't be possible")
      .expect(err_str);
  Ok((device, hardware_clone))
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

#[derive(Default, Clone)]
pub struct TestDeviceCommunicationManagerBuilder {
  devices: WaitingDeviceList,
}

impl TestDeviceCommunicationManagerBuilder {
  pub fn helper(&self) -> TestDeviceCommunicationManagerHelper {
    TestDeviceCommunicationManagerHelper::new(self.devices.clone())
  }
}

impl DeviceCommunicationManagerBuilder for TestDeviceCommunicationManagerBuilder {
  fn finish(&self, sender: Sender<DeviceCommunicationEvent>) -> Box<dyn DeviceCommunicationManager> {
    Box::new(TestDeviceCommunicationManager::new(
      sender,
      self.devices.clone(),
    ))
  }
}

pub struct TestDeviceCommunicationManager {
  device_sender: Sender<DeviceCommunicationEvent>,
  devices: WaitingDeviceList,
  is_scanning: Arc<AtomicBool>
}

impl TestDeviceCommunicationManager {
  pub fn new(device_sender: Sender<DeviceCommunicationEvent>, devices: WaitingDeviceList) -> Self {
    Self {
      device_sender,
      devices,
      is_scanning: Arc::new(AtomicBool::new(false))
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
    let is_scanning = self.is_scanning.clone();
    Box::pin(async move {
      is_scanning.store(true, Ordering::SeqCst);
      let mut devices = devices_vec.lock().await;
      if devices.is_empty() {
        warn!("No devices for test device comm manager to emit, did you mean to do this?");
      }
      while let Some(d) = devices.pop() {      
        let device_name = d.device().as_ref().unwrap().name();  
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
        } else {
          info!("Test DCM emitting device: {}", device_name);
        }
        
      }
      is_scanning.store(false, Ordering::SeqCst);
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

  fn scanning_status(&self) -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
  }
}

#[cfg(test)]
mod test {
  use crate::{
    core::messages::{self, ButtplugMessageSpecVersion, ButtplugServerMessage},
    server::device::hardware::communication::{test::TestDeviceCommunicationManagerBuilder},
    server::ButtplugServerBuilder,
    util::async_manager,
  };
  use futures::StreamExt;

  #[test]
  fn test_test_device_comm_manager() {
    async_manager::block_on(async {
      let mut builder = ButtplugServerBuilder::default();
      let comm_builder = TestDeviceCommunicationManagerBuilder::default();
      let helper = comm_builder.helper();
      builder
        .device_manager_builder()
        .comm_manager(comm_builder);
      let server = builder.finish().expect("Test");
      let recv = server.event_stream();
      pin_mut!(recv);
      let msg =
        messages::RequestServerInfo::new("Test Client", ButtplugMessageSpecVersion::Version2);
      let mut reply = server.parse_message(msg.into()).await;
      assert!(reply.is_ok(), "Should get back ok: {:?}", reply);
      let device = helper.add_ble_device("Massage Demo").await;
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
