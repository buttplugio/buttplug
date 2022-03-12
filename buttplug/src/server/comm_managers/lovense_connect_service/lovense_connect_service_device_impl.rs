use super::lovense_connect_service_comm_manager::LovenseServiceToyInfo;
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::RawReading,
    ButtplugResultFuture,
  },
  device::{
    configuration_manager::{ProtocolDeviceSpecifier, LovenseConnectServiceSpecifier, ProtocolDeviceConfiguration},
    ButtplugDeviceEvent,
    ButtplugDeviceImplCreator,
    DeviceImpl,
    DeviceImplInternal,
    DeviceReadCmd,
    DeviceSubscribeCmd,
    DeviceUnsubscribeCmd,
    DeviceWriteCmd,
    Endpoint,
  },
  util::async_manager,
};
use async_trait::async_trait;
use futures::future::{self, BoxFuture};
use futures_timer::Delay;
use std::{
  fmt::{self, Debug},
  sync::Arc,
  time::Duration,
};
use tokio::sync::{broadcast, RwLock};

pub struct LovenseServiceDeviceImplCreator {
  http_host: String,
  toy_info: Arc<RwLock<LovenseServiceToyInfo>>,
}

impl LovenseServiceDeviceImplCreator {
  pub(super) fn new(http_host: &str, toy_info: Arc<RwLock<LovenseServiceToyInfo>>) -> Self {
    debug!("Emitting a new lovense service device impl creator!");
    Self {
      http_host: http_host.to_owned(),
      toy_info,
    }
  }
}

impl Debug for LovenseServiceDeviceImplCreator {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("LovenseServiceDeviceImplCreator").finish()
  }
}

#[async_trait]
impl ButtplugDeviceImplCreator for LovenseServiceDeviceImplCreator {
  fn specifier(&self) -> ProtocolDeviceSpecifier {
    ProtocolDeviceSpecifier::LovenseConnectService(LovenseConnectServiceSpecifier::default())
  }

  async fn try_create_device_impl(
    &mut self,
    _protocol: ProtocolDeviceConfiguration,
  ) -> Result<DeviceImpl, ButtplugError> {
    let toy_info = self.toy_info.read().await;

    let device_impl_internal =
      LovenseServiceDeviceImpl::new(&self.http_host, self.toy_info.clone(), &toy_info.id);
    let device_impl = DeviceImpl::new(
      &toy_info.name,
      &toy_info.id,
      &[Endpoint::Tx],
      Box::new(device_impl_internal),
    );
    Ok(device_impl)
  }
}

#[derive(Clone, Debug)]
pub struct LovenseServiceDeviceImpl {
  event_sender: broadcast::Sender<ButtplugDeviceEvent>,
  http_host: String,
  toy_info: Arc<RwLock<LovenseServiceToyInfo>>,
}

impl LovenseServiceDeviceImpl {
  fn new(http_host: &str, toy_info: Arc<RwLock<LovenseServiceToyInfo>>, toy_id: &str) -> Self {
    let (device_event_sender, _) = broadcast::channel(256);
    let sender_clone = device_event_sender.clone();
    let toy_id = toy_id.to_owned();
    let toy_info_clone = toy_info.clone();
    async_manager::spawn(async move {
      while toy_info_clone.read().await.connected {
        Delay::new(Duration::from_secs(1)).await;
      }
      let _ = sender_clone.send(ButtplugDeviceEvent::Removed(toy_id));
      info!("Exiting lovense service device connection check loop.");
    });
    Self {
      event_sender: device_event_sender,
      http_host: http_host.to_owned(),
      toy_info,
    }
  }
}

impl DeviceImplInternal for LovenseServiceDeviceImpl {
  fn event_stream(&self) -> broadcast::Receiver<ButtplugDeviceEvent> {
    self.event_sender.subscribe()
  }

  fn connected(&self) -> bool {
    true
  }

  fn disconnect(&self) -> ButtplugResultFuture {
    Box::pin(future::ready(Ok(())))
  }

  // Assume the only thing we'll read is battery.
  fn read_value(
    &self,
    _msg: DeviceReadCmd,
  ) -> BoxFuture<'static, Result<RawReading, ButtplugError>> {
    let toy_info = self.toy_info.clone();
    Box::pin(async move {
      let battery_level = toy_info.read().await.battery.clamp(0, 100) as u8;
      Ok(RawReading::new(0, Endpoint::Rx, vec![battery_level]))
    })
  }

  fn write_value(&self, msg: DeviceWriteCmd) -> ButtplugResultFuture {
    let command_url = format!(
      "{}/{}",
      self.http_host,
      std::str::from_utf8(&msg.data)
        .expect("We build this in the protocol then have to serialize to [u8], but it's a string.")
    );
    Box::pin(async move {
      match reqwest::get(command_url).await {
        Ok(_) => Ok(()),
        Err(err) => {
          error!("Got http error: {}", err);
          Err(ButtplugDeviceError::UnhandledCommand(err.to_string()).into())
        }
      }
    })
  }

  fn subscribe(&self, _msg: DeviceSubscribeCmd) -> ButtplugResultFuture {
    panic!("We should never get here!");
  }

  fn unsubscribe(&self, _msg: DeviceUnsubscribeCmd) -> ButtplugResultFuture {
    panic!("We should never get here!");
  }
}
