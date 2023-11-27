// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::lovense_connect_service_comm_manager::{get_local_info, LovenseServiceToyInfo};
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    configuration::{LovenseConnectServiceSpecifier, ProtocolCommunicationSpecifier},
    hardware::{
      GenericHardwareSpecializer,
      Hardware,
      HardwareConnector,
      HardwareEvent,
      HardwareInternal,
      HardwareReadCmd,
      HardwareReading,
      HardwareSpecializer,
      HardwareSubscribeCmd,
      HardwareUnsubscribeCmd,
      HardwareWriteCmd,
    },
  },
  util::async_manager,
};
use async_trait::async_trait;
use futures::future::{self, BoxFuture, FutureExt};
use std::{
  fmt::{self, Debug},
  sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
  },
  time::Duration,
};
use tokio::sync::broadcast;

pub struct LovenseServiceHardwareConnector {
  http_host: String,
  toy_info: LovenseServiceToyInfo,
}

impl LovenseServiceHardwareConnector {
  pub(super) fn new(http_host: &str, toy_info: &LovenseServiceToyInfo) -> Self {
    debug!("Emitting a new lovense service hardware connector!");
    Self {
      http_host: http_host.to_owned(),
      toy_info: toy_info.clone(),
    }
  }
}

impl Debug for LovenseServiceHardwareConnector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("LovenseServiceHardwareConnector").finish()
  }
}

#[async_trait]
impl HardwareConnector for LovenseServiceHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    ProtocolCommunicationSpecifier::LovenseConnectService(LovenseConnectServiceSpecifier::default())
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    let hardware_internal = LovenseServiceHardware::new(&self.http_host, &self.toy_info.id);
    let hardware = Hardware::new(
      &self.toy_info.name,
      &self.toy_info.id,
      &[Endpoint::Tx],
      Box::new(hardware_internal),
    );
    Ok(Box::new(GenericHardwareSpecializer::new(hardware)))
  }
}

#[derive(Clone, Debug)]
pub struct LovenseServiceHardware {
  event_sender: broadcast::Sender<HardwareEvent>,
  http_host: String,
  battery_level: Arc<AtomicU8>,
}

impl LovenseServiceHardware {
  fn new(http_host: &str, toy_id: &str) -> Self {
    let (device_event_sender, _) = broadcast::channel(256);
    let sender_clone = device_event_sender.clone();
    let toy_id = toy_id.to_owned();
    let host = http_host.to_owned();
    let battery_level = Arc::new(AtomicU8::new(100));
    let battery_level_clone = battery_level.clone();
    async_manager::spawn(async move {
      loop {
        // SutekhVRC/VibeCheck patch for delay because Lovense Connect HTTP servers crash (Perma DOS)
        tokio::time::sleep(Duration::from_secs(1)).await;
        match get_local_info(&host).await {
          Some(info) => {
            for (_, toy) in info.data.iter() {
              if toy.id != toy_id {
                continue;
              }
              if !toy.connected {
                let _ = sender_clone.send(HardwareEvent::Disconnected(toy_id.clone()));
                info!("Exiting lovense service device connection check loop.");
                break;
              }
              battery_level_clone.store(toy.battery.clamp(0, 100) as u8, Ordering::SeqCst);
              break;
            }
          }
          None => {
            let _ = sender_clone.send(HardwareEvent::Disconnected(toy_id.clone()));
            info!("Exiting lovense service device connection check loop.");
            break;
          }
        }
      }
    });
    Self {
      event_sender: device_event_sender,
      http_host: http_host.to_owned(),
      battery_level,
    }
  }
}

impl HardwareInternal for LovenseServiceHardware {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.event_sender.subscribe()
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Ok(())).boxed()
  }

  // Assume the only thing we'll read is battery.
  fn read_value(
    &self,
    _msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    let battery_level = self.battery_level.clone();
    async move {
      Ok(HardwareReading::new(
        Endpoint::Rx,
        &[battery_level.load(Ordering::SeqCst)],
      ))
    }
    .boxed()
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let command_url = format!(
      "{}/{}",
      self.http_host,
      std::str::from_utf8(&msg.data)
        .expect("We build this in the protocol then have to serialize to [u8], but it's a string.")
    );

    trace!("Sending Lovense Connect command: {}", command_url);
    async move {
      match reqwest::get(command_url).await {
        Ok(res) => {
          async_manager::spawn(async move {
            trace!(
              "Got http response: {}",
              res.text().await.unwrap_or(format!("no response"))
            );
          });
          Ok(())
        }
        Err(err) => {
          error!("Got http error: {}", err);
          Err(ButtplugDeviceError::UnhandledCommand(err.to_string()))
        }
      }
    }
    .boxed()
  }

  fn subscribe(
    &self,
    _msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "Lovense Connect does not support subscribe".to_owned(),
    )))
    .boxed()
  }

  fn unsubscribe(
    &self,
    _msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "Lovense Connect does not support unsubscribe".to_owned(),
    )))
    .boxed()
  }
}
