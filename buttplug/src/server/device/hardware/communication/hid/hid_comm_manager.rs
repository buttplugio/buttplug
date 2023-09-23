use crate::{
  core::errors::ButtplugDeviceError,
  server::device::hardware::communication::{
    HardwareCommunicationManager,
    HardwareCommunicationManagerBuilder,
    HardwareCommunicationManagerEvent,
    TimedRetryCommunicationManager,
    TimedRetryCommunicationManagerImpl,
  },
};
use async_trait::async_trait;
use hidapi::HidApi;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::hid_device_impl::HidHardwareConnector;

#[derive(Default)]
pub struct HidCommunicationManagerBuilder {}

impl HardwareCommunicationManagerBuilder for HidCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(TimedRetryCommunicationManager::new(
      HidCommunicationManager::new(sender),
    ))
  }
}

pub struct HidCommunicationManager {
  sender: Sender<HardwareCommunicationManagerEvent>,
  hidapi: Arc<HidApi>,
}

impl HidCommunicationManager {
  fn new(sender: Sender<HardwareCommunicationManagerEvent>) -> Self {
    Self {
      sender,
      hidapi: Arc::new(HidApi::new().unwrap()),
    }
  }
}

#[async_trait]
impl TimedRetryCommunicationManagerImpl for HidCommunicationManager {
  fn name(&self) -> &'static str {
    "HIDCommunicationManager"
  }

  async fn scan(&self) -> Result<(), ButtplugDeviceError> {
    // TODO Does this block? Should it run in one of our threads?
    let device_sender = self.sender.clone();
    let api = self.hidapi.clone();

    let mut seen_addresses = vec![];
    for device in api.device_list() {
      if let None = device.serial_number() {
        continue;
      }
      let serial_number = device.serial_number().unwrap().to_owned();
      if seen_addresses.contains(&serial_number) {
        continue;
      }
      seen_addresses.push(serial_number.clone());
      let device_creator = HidHardwareConnector::new(api.clone(), &device);
      if device_sender
        .send(HardwareCommunicationManagerEvent::DeviceFound {
          name: device.product_string().unwrap().to_owned(),
          address: serial_number,
          creator: Box::new(device_creator),
        })
        .await
        .is_err()
      {
        error!("Device manager receiver dropped, cannot send device found message.");
        return Ok(());
      }
    }
    Ok(())
  }

  fn can_scan(&self) -> bool {
    true
  }
}
