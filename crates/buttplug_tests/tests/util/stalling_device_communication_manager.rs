// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! A test comm manager whose device bringup stalls forever in `connect()`.
//!
//! On `start_scanning` it emits a single `DeviceFound` event carrying a
//! [StallingHardwareConnector]. The device-manager event loop spawns a bringup
//! task that awaits `connect()` — which never resolves — modelling a real BLE
//! connect that hangs. This is the precise scenario the cancellable-bringup fix
//! guards: without a `biased` select on the bringup token, `shutdown()`'s
//! `wait_empty_under` would block forever waiting for the bringup task to
//! deregister.

use async_trait::async_trait;
use buttplug_core::{ButtplugResultFuture, errors::ButtplugDeviceError};
use buttplug_server::device::hardware::{
  HardwareConnector,
  HardwareSpecializer,
  communication::{
    HardwareCommunicationManager,
    HardwareCommunicationManagerBuilder,
    HardwareCommunicationManagerEvent,
  },
};
use buttplug_server_device_config::{BluetoothLESpecifier, ProtocolCommunicationSpecifier};
use futures::FutureExt;
use log::error;
use std::collections::HashMap;
use tokio::sync::{Notify, mpsc::Sender};

/// Shared state used to deterministically observe that bring-up entered connect().
#[derive(Clone, Debug, Default)]
pub struct StallingDeviceState {
  pub connect_started: std::sync::Arc<Notify>,
}

/// A `HardwareConnector` whose `connect()` future never resolves.
#[derive(Debug)]
struct StallingHardwareConnector {
  specifier: ProtocolCommunicationSpecifier,
  state: StallingDeviceState,
}

#[async_trait]
impl HardwareConnector for StallingHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    self.specifier.clone()
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    // Block forever: this models a device connect that hangs and never returns.
    // The bringup task must drop this future when its cancellation token fires.
    self.state.connect_started.notify_waiters();
    std::future::pending::<()>().await;
    unreachable!("stalling connector connect() should never resolve");
  }
}

#[derive(Clone, Default)]
pub struct StallingDeviceCommunicationManagerBuilder {
  state: StallingDeviceState,
}

impl StallingDeviceCommunicationManagerBuilder {
  pub fn state(&self) -> StallingDeviceState {
    self.state.clone()
  }
}

impl HardwareCommunicationManagerBuilder for StallingDeviceCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(StallingDeviceCommunicationManager {
      device_sender: sender,
      state: self.state.clone(),
    })
  }
}

pub struct StallingDeviceCommunicationManager {
  device_sender: Sender<HardwareCommunicationManagerEvent>,
  state: StallingDeviceState,
}

impl HardwareCommunicationManager for StallingDeviceCommunicationManager {
  fn name(&self) -> &'static str {
    "StallingDeviceCommunicationManager"
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    let device_sender = self.device_sender.clone();
    let state = self.state.clone();
    async move {
      // "Massage Demo" is a known test device name with a real protocol config,
      // so bringup proceeds into connect() (which then stalls).
      let specifier = ProtocolCommunicationSpecifier::BluetoothLE(
        BluetoothLESpecifier::new_from_device("Massage Demo", &HashMap::new(), &[]),
      );
      let connector = StallingHardwareConnector {
        specifier,
        state: state.clone(),
      };
      if device_sender
        .send(HardwareCommunicationManagerEvent::DeviceFound {
          name: "Massage Demo".to_owned(),
          address: "stalling-device-0".to_owned(),
          creator: Box::new(connector),
        })
        .await
        .is_err()
      {
        error!("Device channel no longer open.");
      }
      Ok(())
    }
    .boxed()
  }

  fn stop_scanning(&mut self) -> ButtplugResultFuture {
    async { Ok(()) }.boxed()
  }

  fn scanning_status(&self) -> bool {
    false
  }

  fn can_scan(&self) -> bool {
    true
  }
}
