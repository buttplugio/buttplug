// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#[cfg(feature = "btleplug-manager")]
pub mod btleplug;
#[cfg(feature = "lovense-connect-service-manager")]
pub mod lovense_connect_service;
#[cfg(feature = "lovense-dongle-manager")]
pub mod lovense_dongle;
#[cfg(feature = "serial-manager")]
pub mod serialport;
#[cfg(all(feature = "xinput-manager", target_os = "windows"))]
pub mod xinput;

#[cfg(feature = "websocket-server-manager")]
pub mod websocket_server;

pub mod test;

use crate::{core::ButtplugResultFuture, server::device::hardware::device_impl::HardwareCreator};
use serde::{Deserialize, Serialize};
use std::sync::{atomic::AtomicBool, Arc};
use thiserror::Error;
use tokio::sync::mpsc::Sender;

#[derive(Debug)]
pub enum DeviceCommunicationEvent {
  // This event only means that a device has been found. The work still needs
  // to be done to make sure we can use it.
  DeviceFound {
    name: String,
    address: String,
    creator: Box<dyn HardwareCreator>,
  },
  DeviceManagerAdded(Arc<AtomicBool>),
  ScanningStarted,
  ScanningFinished,
}

pub trait DeviceCommunicationManagerBuilder: Send {
  fn finish(&self, sender: Sender<DeviceCommunicationEvent>) -> Box<dyn DeviceCommunicationManager>;
}

pub trait DeviceCommunicationManager: Send + Sync {
  fn name(&self) -> &'static str;
  fn start_scanning(&self) -> ButtplugResultFuture;
  fn stop_scanning(&self) -> ButtplugResultFuture;
  fn scanning_status(&self) -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
  }
  fn can_scan(&self) -> bool;
  // Events happen via channel senders passed to the comm manager.
}

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ButtplugDeviceSpecificError {
  // XInput library doesn't derive error on its error enum. :(
  #[cfg(all(feature = "xinput-manager", target_os = "windows"))]
  #[error("XInput usage error: {0}")]
  XInputError(String),
  // Btleplug library uses Failure, not Error, on its error enum. :(
  #[cfg(feature = "btleplug-manager")]
  #[error("Btleplug error: {0}")]
  BtleplugError(String),
  #[cfg(feature = "serial-manager")]
  #[error("Serial error: {0}")]
  SerialError(String),
}
