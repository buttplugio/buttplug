use super::TestDeviceImplCreator;
use crate::{
  core::ButtplugResultFuture,
  server::comm_managers::{
    DeviceCommunicationEvent, DeviceCommunicationManager, DeviceCommunicationManagerCreator,
  },
};
use std::sync::Arc;
use async_mutex::Mutex;
use async_channel::Sender;
use futures::future;

pub struct TestDeviceCommunicationManager {
  device_sender: Sender<DeviceCommunicationEvent>,
  devices: Arc<Mutex<Vec<TestDeviceImplCreator>>>,
}

impl TestDeviceCommunicationManager {
  pub fn get_devices_clone(&self) -> Arc<Mutex<Vec<TestDeviceImplCreator>>> {
    self.devices.clone()
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
        device_sender
          .send(DeviceCommunicationEvent::DeviceFound(Box::new(d)))
          .await;
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
    device::DeviceImpl,
    server::ButtplugServer,
    test::TestDevice,
    util::async_manager
  };
  use futures::StreamExt;

  #[test]
  fn test_test_device_comm_manager() {
    let _ = env_logger::builder().is_test(true).try_init();
    let (mut server, mut recv) = ButtplugServer::new("Test Server", 0);
    let (device, device_creator) =
      TestDevice::new_bluetoothle_test_device_impl_creator("Massage Demo");

    async_manager::block_on(async {
      let devices = server.add_test_comm_manager();
      devices.lock().await.push(device_creator);
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
