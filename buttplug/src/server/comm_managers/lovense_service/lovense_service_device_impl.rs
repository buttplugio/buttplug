
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::RawReading,
    ButtplugResultFuture,
  },
  device::{
    configuration_manager::{DeviceSpecifier, ProtocolDefinition, LovenseServiceSpecifier},
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
  server::comm_managers::ButtplugDeviceSpecificError,
};
use async_trait::async_trait;
use futures::future::{self, BoxFuture};
use std::{
  fmt::{self, Debug},
};
use tokio::sync::broadcast;

pub struct LovenseServiceDeviceImplCreator {
  http_host: String,
  http_port: u16,
  toy_name: String,
  toy_id: String,
}

impl LovenseServiceDeviceImplCreator {
  pub fn new(http_host: &str, http_port: u16, toy_name: &str, toy_id: &str) -> Self {
    debug!("Emitting a new lovense service device impl creator!");
    Self { 
      http_host: http_host.to_owned(),
      http_port,
      toy_name: toy_name.to_owned(),
      toy_id: toy_id.to_owned(),
    }
  }
}

impl Debug for LovenseServiceDeviceImplCreator {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("LovenseServiceDeviceImplCreator")
      .finish()
  }
}

#[async_trait]
impl ButtplugDeviceImplCreator for LovenseServiceDeviceImplCreator {
  fn get_specifier(&self) -> DeviceSpecifier {
    DeviceSpecifier::LovenseService(LovenseServiceSpecifier::default())
  }

  async fn try_create_device_impl(
    &mut self,
    _protocol: ProtocolDefinition,
  ) -> Result<DeviceImpl, ButtplugError> {
    let device_impl_internal = LovenseServiceDeviceImpl::new(&self.http_host, self.http_port, &self.toy_name, &self.toy_id);
    let device_impl = DeviceImpl::new(
      &self.toy_name,
      &self.toy_id,
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
  http_port: u16,
  toy_name: String,
  toy_id: String,
}

impl LovenseServiceDeviceImpl {
  pub fn new(http_host: &str, http_port: u16, toy_name: &str, toy_id: &str) -> Self {
    let (device_event_sender, _) = broadcast::channel(256);
    Self {
      event_sender: device_event_sender,
      http_host: http_host.to_owned(),
      http_port,
      toy_name: toy_name.to_owned(),
      toy_id: toy_id.to_owned(),
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

  fn read_value(
    &self,
    _msg: DeviceReadCmd,
  ) -> BoxFuture<'static, Result<RawReading, ButtplugError>> {
    panic!("We should never get here!");
  }

  fn write_value(&self, msg: DeviceWriteCmd) -> ButtplugResultFuture {
    panic!("We should never get here!");
  }

  fn subscribe(&self, _msg: DeviceSubscribeCmd) -> ButtplugResultFuture {
    panic!("We should never get here!");
  }

  fn unsubscribe(&self, _msg: DeviceUnsubscribeCmd) -> ButtplugResultFuture {
    panic!("We should never get here!");
  }
}
