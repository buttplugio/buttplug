// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2025 Nonpolynomial Labs LLC., Milibyte LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::UdpHardwareConnector;
use buttplug_core::{ ButtplugResultFuture};
use buttplug_server::device::hardware::communication::{
  HardwareCommunicationManager,
  HardwareCommunicationManagerBuilder,
  HardwareCommunicationManagerEvent,
};
use buttplug_server_device_config::UdpSpecifier;
use futures::{FutureExt};
use tokio::sync::mpsc::Sender;

#[derive(Default, Clone)]
pub struct UdpCommunicationManagerBuilder {}

impl HardwareCommunicationManagerBuilder for UdpCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(UdpCommunicationManager::new(sender))
  }
}

pub struct UdpCommunicationManager {
  sender: Sender<HardwareCommunicationManagerEvent>
}

impl UdpCommunicationManager {
  pub fn new(sender: Sender<HardwareCommunicationManagerEvent>) -> Self {
    trace!("Udp socket created.");
    Self { sender, }
  }
}

impl HardwareCommunicationManager for UdpCommunicationManager {
  fn name(&self) -> &'static str {
    "UdpCommunicationManager"
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    debug!("Udp scan starting");
    let sender_clone = self.sender.clone();
    async move {
      // TODO: Look through confiuration to locate configured UDP
      let specifiers = [
        UdpSpecifier::new("192.168.2.185", 8000)
      ];
      for specifier in specifiers
      {
        if sender_clone.send(HardwareCommunicationManagerEvent::DeviceFound {
          name: format!("UDP Device {}", specifier.to_string()),
          address: specifier.to_string(),
          creator: Box::new(UdpHardwareConnector::new(
            specifier
          )),
        })
        .await
        .is_err()
        {
          error!("Device manager disappeared, exiting");
        }
      }
      Ok(())
    }.boxed()
  }

  fn stop_scanning(&mut self) -> ButtplugResultFuture {
    debug!("Udp scan stopping");
    async move { Ok(()) }.boxed()
  }

  fn can_scan(&self) -> bool {
    true
  }
}

