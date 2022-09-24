// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::time::Duration;

use super::SerialPortHardwareConnector;
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
use serialport::available_ports;
use tokio::sync::mpsc::Sender;

#[derive(Default, Clone)]
pub struct SerialPortCommunicationManagerBuilder {}

impl HardwareCommunicationManagerBuilder for SerialPortCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(TimedRetryCommunicationManager::new(
      SerialPortCommunicationManager::new(sender),
    ))
  }
}

pub struct SerialPortCommunicationManager {
  sender: Sender<HardwareCommunicationManagerEvent>,
}

impl SerialPortCommunicationManager {
  fn new(sender: Sender<HardwareCommunicationManagerEvent>) -> Self {
    trace!("Serial port created.");
    Self { sender }
  }
}

#[async_trait]
impl TimedRetryCommunicationManagerImpl for SerialPortCommunicationManager {
  fn name(&self) -> &'static str {
    "SerialPortCommunicationManager"
  }

  fn rescan_wait_duration(&self) -> Duration {
    Duration::from_secs(5)
  }

  async fn scan(&self) -> Result<(), ButtplugDeviceError> {
    trace!("Serial port manager scanning for devices.");
    match available_ports() {
      Ok(ports) => {
        debug!("Got {} serial ports back", ports.len());
        for p in ports {
          trace!(
            "Sending serial port {:?} for possible device connection.",
            p
          );
          if self
            .sender
            .send(HardwareCommunicationManagerEvent::DeviceFound {
              name: format!("Serial Port Device {}", p.port_name),
              address: p.port_name.clone(),
              creator: Box::new(SerialPortHardwareConnector::new(&p)),
            })
            .await
            .is_err()
          {
            debug!("Device manager disappeared, exiting.");
            break;
          }
        }
      }
      Err(_) => {
        trace!("No serial ports found");
      }
    }
    Ok(())
  }

  // We should always be able to at least look at serial ports.
  fn can_scan(&self) -> bool {
    true
  }
}
