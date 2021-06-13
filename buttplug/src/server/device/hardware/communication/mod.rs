// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

// Network DCMs work on all platforms
#[cfg(feature = "lovense-connect-service-manager")]
pub mod lovense_connect_service;
#[cfg(feature = "websocket-server-manager")]
pub mod websocket_server;

// BTLEPlug works on anything not WASM
#[cfg(all(
  feature = "btleplug-manager",
  any(
    target_os = "windows",
    target_os = "macos",
    target_os = "linux",
    target_os = "ios",
    target_os = "android"
  )
))]
pub mod btleplug;

// Lovense Dongles and Serial Ports work on all desktop platforms
#[cfg(all(
  feature = "lovense-dongle-manager",
  any(target_os = "windows", target_os = "macos", target_os = "linux")
))]
pub mod lovense_dongle;
#[cfg(all(
  feature = "serial-manager",
  any(target_os = "windows", target_os = "macos", target_os = "linux")
))]
pub mod serialport;

#[cfg(all(
  feature = "hid-manager",
  any(target_os = "windows", target_os = "macos", target_os = "linux")
))]
pub mod hid;

// XInput is windows only
#[cfg(all(feature = "xinput-manager", target_os = "windows"))]
pub mod xinput;

use crate::{
  core::{errors::ButtplugDeviceError, ButtplugResultFuture},
  server::device::hardware::HardwareConnector,
  util::{async_manager, sleep},
};
use async_trait::async_trait;
use futures::future::{self, FutureExt};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use thiserror::Error;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub enum HardwareCommunicationManagerEvent {
  // This event only means that a device has been found. The work still needs
  // to be done to make sure we can use it.
  DeviceFound {
    name: String,
    address: String,
    creator: Box<dyn HardwareConnector>,
  },
  ScanningFinished,
}

pub trait HardwareCommunicationManagerBuilder: Send {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager>;
}

pub trait HardwareCommunicationManager: Send + Sync {
  fn name(&self) -> &'static str;
  fn start_scanning(&mut self) -> ButtplugResultFuture;
  fn stop_scanning(&mut self) -> ButtplugResultFuture;
  fn scanning_status(&self) -> bool {
    false
  }
  fn can_scan(&self) -> bool;
  // Events happen via channel senders passed to the comm manager.
}

#[derive(Error, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HardwareSpecificError {
  // XInput library doesn't derive error on its error enum. :(
  #[cfg(all(feature = "xinput-manager", target_os = "windows"))]
  #[error("XInput usage error: {0}")]
  XInputError(String),
  // Btleplug library uses Failure, not Error, on its error enum. :(
  #[cfg(all(
    feature = "btleplug-manager",
    any(
      target_os = "windows",
      target_os = "macos",
      target_os = "linux",
      target_os = "ios",
      target_os = "android"
    )
  ))]
  #[error("Btleplug error: {0}")]
  BtleplugError(String),
  #[cfg(all(
    feature = "serial-manager",
    any(target_os = "windows", target_os = "macos", target_os = "linux")
  ))]
  #[error("Serial error: {0}")]
  SerialError(String),
}

#[async_trait]
pub trait TimedRetryCommunicationManagerImpl: Sync + Send {
  fn name(&self) -> &'static str;
  fn can_scan(&self) -> bool;
  fn rescan_wait_duration(&self) -> Duration {
    Duration::from_secs(1)
  }
  async fn scan(&self) -> Result<(), ButtplugDeviceError>;
}

pub struct TimedRetryCommunicationManager<T: TimedRetryCommunicationManagerImpl + 'static> {
  comm_manager: Arc<T>,
  cancellation_token: Option<CancellationToken>,
}

impl<T: TimedRetryCommunicationManagerImpl> TimedRetryCommunicationManager<T> {
  pub fn new(comm_manager: T) -> Self {
    Self {
      comm_manager: Arc::new(comm_manager),
      cancellation_token: None,
    }
  }
}

impl<T: TimedRetryCommunicationManagerImpl> HardwareCommunicationManager
  for TimedRetryCommunicationManager<T>
{
  fn name(&self) -> &'static str {
    self.comm_manager.name()
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    if self.cancellation_token.is_some() {
      return future::ready(Ok(())).boxed();
    }
    let comm_manager = self.comm_manager.clone();
    let token = CancellationToken::new();
    let child_token = token.child_token();
    self.cancellation_token = Some(token);
    let duration = self.comm_manager.rescan_wait_duration();
    async move {
      async_manager::spawn(async move {
        loop {
          if let Err(err) = comm_manager.scan().await {
            error!("Timed Device Communication Manager Failure: {}", err);
            break;
          }
          tokio::select! {
            _ = sleep(duration) => continue,
            _ = child_token.cancelled() => break,
          }
        }
      });
      Ok(())
    }
    .boxed()
  }

  fn stop_scanning(&mut self) -> ButtplugResultFuture {
    if self.cancellation_token.is_none() {
      return future::ready(Ok(())).boxed();
    }
    self.cancellation_token.take().unwrap().cancel();
    future::ready(Ok(())).boxed()
  }

  fn scanning_status(&self) -> bool {
    self.cancellation_token.is_some()
  }
  fn can_scan(&self) -> bool {
    self.comm_manager.can_scan()
  }
}

impl<T: TimedRetryCommunicationManagerImpl> Drop for TimedRetryCommunicationManager<T> {
  fn drop(&mut self) {
    // We set the cancellation token without doing anything with the future, so we're fine to ignore
    // the return.
    let _ = self.stop_scanning();
  }
}
