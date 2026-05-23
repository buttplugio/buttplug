// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server::device::hardware::communication::{
  HardwareCommunicationManager,
  HardwareCommunicationManagerBuilder,
  HardwareCommunicationManagerEvent,
  TimedRetryCommunicationManager,
  TimedRetryCommunicationManagerImpl,
};
use hidapi::{HidApi, HidResult};
use log::*;
use std::sync::{Arc, Mutex};
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
  hidapi: Mutex<Option<Arc<HidApi>>>,
  hidapi_factory: Box<dyn Fn() -> HidResult<HidApi> + Send + Sync>,
}

impl HidCommunicationManager {
  fn new(sender: Sender<HardwareCommunicationManagerEvent>) -> Self {
    Self::new_with_hidapi_factory(sender, Box::new(HidApi::new))
  }

  fn new_with_hidapi_factory(
    sender: Sender<HardwareCommunicationManagerEvent>,
    hidapi_factory: Box<dyn Fn() -> HidResult<HidApi> + Send + Sync>,
  ) -> Self {
    Self {
      sender,
      hidapi: Mutex::new(None),
      hidapi_factory,
    }
  }

  fn hidapi(&self) -> Result<Arc<HidApi>, ButtplugDeviceError> {
    let mut hidapi = self.hidapi.lock().map_err(|_| {
      ButtplugDeviceError::DeviceCommunicationError("HIDAPI lock poisoned.".to_owned())
    })?;
    if let Some(api) = hidapi.as_ref() {
      return Ok(api.clone());
    }

    let api = (self.hidapi_factory)().map_err(|err| {
      error!("Failed to create HIDAPI instance: {}", err);
      ButtplugDeviceError::DeviceConnectionError(format!("Cannot create HIDAPI: {err}"))
    })?;
    let api = Arc::new(api);
    *hidapi = Some(api.clone());
    Ok(api)
  }

  #[cfg(target_os = "macos")]
  fn hidapi_initialized(&self) -> bool {
    self.hidapi.lock().map(|api| api.is_some()).unwrap_or(false)
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
    let api = self.hidapi()?;

    let mut seen_addresses = vec![];
    for device in api.device_list() {
      let Some(serial_number) = device.serial_number().map(str::to_owned) else {
        continue;
      };
      if seen_addresses.contains(&serial_number) {
        continue;
      }
      seen_addresses.push(serial_number.clone());
      let name = device.product_string().unwrap_or("Unknown HID Device");
      let device_creator = HidHardwareConnector::new(api.clone(), device);
      if device_sender
        .send(HardwareCommunicationManagerEvent::DeviceFound {
          name: name.to_owned(),
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

#[cfg(target_os = "macos")]
fn reset_hidapi() {
  unsafe extern "C" {
    fn hid_exit() -> std::os::raw::c_int;
  }
  unsafe {
    hid_exit();
  }
}

impl Drop for HidCommunicationManager {
  fn drop(&mut self) {
    #[cfg(target_os = "macos")]
    if self.hidapi_initialized() {
      reset_hidapi();
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use hidapi::HidError;
  use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  };

  #[test]
  fn construction_does_not_initialize_hidapi() {
    let (sender, _receiver) = tokio::sync::mpsc::channel(1);
    let called = Arc::new(AtomicBool::new(false));
    let factory_called = called.clone();

    let _manager = HidCommunicationManager::new_with_hidapi_factory(
      sender,
      Box::new(move || {
        factory_called.store(true, Ordering::Relaxed);
        Err(HidError::InitializationError)
      }),
    );

    assert!(!called.load(Ordering::Relaxed));
  }

  #[test]
  fn scan_returns_error_when_hidapi_initialization_fails() {
    let (sender, _receiver) = tokio::sync::mpsc::channel(1);
    let manager = HidCommunicationManager::new_with_hidapi_factory(
      sender,
      Box::new(|| Err(HidError::InitializationError)),
    );

    let result = futures::executor::block_on(manager.scan());

    assert!(matches!(
      result,
      Err(ButtplugDeviceError::DeviceConnectionError(message))
        if message.contains("Cannot create HIDAPI")
    ));
  }
}
