// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::xinput_hardware::XInputHardwareConnector;
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
use rusty_xinput::XInputHandle;
use std::string::ToString;
use tokio::sync::mpsc;

// 1-index this because we use it elsewhere for showing which controller is which.
#[derive(Debug, Display, Clone, Copy)]
#[repr(u8)]
pub enum XInputControllerIndex {
  XInputController1 = 0,
  XInputController2 = 1,
  XInputController3 = 2,
  XInputController4 = 3,
}

#[derive(Default, Clone)]
pub struct XInputDeviceCommunicationManagerBuilder {}

impl HardwareCommunicationManagerBuilder for XInputDeviceCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: mpsc::Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(TimedRetryCommunicationManager::new(
      XInputDeviceCommunicationManager::new(sender),
    ))
  }
}

pub struct XInputDeviceCommunicationManager {
  sender: mpsc::Sender<HardwareCommunicationManagerEvent>,
  handle: XInputHandle,
}

impl XInputDeviceCommunicationManager {
  fn new(sender: mpsc::Sender<HardwareCommunicationManagerEvent>) -> Self {
    Self {
      sender,
      handle: rusty_xinput::XInputHandle::load_default()
        .expect("Always loads in windows, this shouldn't run elsewhere."),
    }
  }
}

#[async_trait]
impl TimedRetryCommunicationManagerImpl for XInputDeviceCommunicationManager {
  fn name(&self) -> &'static str {
    "XInputDeviceCommunicationManager"
  }

  async fn scan(&self) -> Result<(), ButtplugDeviceError> {
    trace!("XInput manager scanning for devices");
    for i in &[
      XInputControllerIndex::XInputController1,
      XInputControllerIndex::XInputController2,
      XInputControllerIndex::XInputController3,
      XInputControllerIndex::XInputController4,
    ] {
      match self.handle.get_state(*i as u32) {
        Ok(_) => {
          let index = *i as u32;
          debug!("XInput manager found device {}", index);
          let device_creator = Box::new(XInputHardwareConnector::new(*i));

          if self
            .sender
            .send(HardwareCommunicationManagerEvent::DeviceFound {
              name: i.to_string(),
              address: i.to_string(),
              creator: device_creator,
            })
            .await
            .is_err()
          {
            error!("Error sending device found message from Xinput.");
            break;
          }
        }
        Err(_) => {
          continue;
        }
      }
    }
    Ok(())
  }

  // We should always be able to at least look at xinput if we're up on windows.
  fn can_scan(&self) -> bool {
    true
  }
}
