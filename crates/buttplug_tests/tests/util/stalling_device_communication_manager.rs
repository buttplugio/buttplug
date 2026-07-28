// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

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
use tokio::sync::mpsc::Sender;

#[derive(Debug)]
struct StallingHardwareConnector {
  specifier: ProtocolCommunicationSpecifier,
}

#[async_trait]
impl HardwareConnector for StallingHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    self.specifier.clone()
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    std::future::pending::<()>().await;
    unreachable!("stalling connector connect() should never resolve");
  }
}

#[derive(Default)]
pub struct StallingDeviceCommunicationManagerBuilder;

impl HardwareCommunicationManagerBuilder for StallingDeviceCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(StallingDeviceCommunicationManager {
      device_sender: sender,
    })
  }
}

struct StallingDeviceCommunicationManager {
  device_sender: Sender<HardwareCommunicationManagerEvent>,
}

impl HardwareCommunicationManager for StallingDeviceCommunicationManager {
  fn name(&self) -> &'static str {
    "StallingDeviceCommunicationManager"
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    let device_sender = self.device_sender.clone();
    async move {
      let specifier = ProtocolCommunicationSpecifier::BluetoothLE(
        BluetoothLESpecifier::new_from_device("Massage Demo", &HashMap::new(), &[]),
      );
      let connector = StallingHardwareConnector { specifier };
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
