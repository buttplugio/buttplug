// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::time::Duration;

use super::UdpHardwareConnector;
use async_trait::async_trait;
use buttplug_core::{util::async_manager, errors::ButtplugDeviceError, ButtplugResultFuture};
use buttplug_server::device::hardware::communication::{
  HardwareCommunicationManager,
  HardwareCommunicationManagerBuilder,
  HardwareCommunicationManagerEvent,
};
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
  sender: Sender<HardwareCommunicationManagerEvent>,
}

impl UdpCommunicationManager {
  fn new(sender: Sender<HardwareCommunicationManagerEvent>) -> Self {
    trace!("Udp socket created.");
    Self { sender }
  }
}

impl HardwareCommunicationManager for UdpCommunicationManager {
  fn name(&self) -> &'static str {
    "UdpCommunicationManager"
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    debug!("Udp scan: noop");
    async move { Ok(()) }.boxed()
  }

  fn stop_scanning(&mut self) -> ButtplugResultFuture {
    async move { Ok(()) }.boxed()
  }

  fn can_scan(&self) -> bool {
    true
  }
}
