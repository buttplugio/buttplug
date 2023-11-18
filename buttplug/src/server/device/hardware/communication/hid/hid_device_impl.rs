use super::hidapi_async::HidAsyncDevice;
use crate::{
  core::errors::ButtplugDeviceError,
  server::device::{
    configuration::{HIDSpecifier, ProtocolCommunicationSpecifier},
    hardware::{
      Endpoint,
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
};
use async_trait::async_trait;
use futures::{future::BoxFuture, AsyncWriteExt};
use hidapi::{DeviceInfo, HidApi};
use std::{
  fmt::{self, Debug},
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use tokio::sync::{broadcast, Mutex};

pub struct HidHardwareConnector {
  hid_instance: Arc<HidApi>,
  device_info: DeviceInfo,
}

impl HidHardwareConnector {
  pub fn new(hid_instance: Arc<HidApi>, device_info: &DeviceInfo) -> Self {
    Self {
      hid_instance,
      device_info: device_info.clone(),
    }
  }
}

impl Debug for HidHardwareConnector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("HIDHardwareConnector")
      .field("vid", &self.device_info.vendor_id())
      .field("pid", &self.device_info.product_id())
      .finish()
  }
}

#[async_trait]
impl HardwareConnector for HidHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    info!(
      "Specifier for {}: {:#04x} {:#04x}",
      self.device_info.product_string().unwrap(),
      self.device_info.vendor_id(),
      self.device_info.product_id()
    );
    ProtocolCommunicationSpecifier::HID(HIDSpecifier::new(
      self.device_info.vendor_id(),
      self.device_info.product_id(),
    ))
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    let device = self.device_info.open_device(&self.hid_instance).unwrap();
    let device_impl_internal = HIDDeviceImpl::new(HidAsyncDevice::new(device).unwrap());
    info!(
      "New HID device created: {}",
      self.device_info.product_string().unwrap()
    );
    let hardware = Hardware::new(
      &self.device_info.product_string().unwrap(),
      &self.device_info.serial_number().unwrap(),
      &[Endpoint::Rx, Endpoint::Tx],
      Box::new(device_impl_internal),
    );
    Ok(Box::new(GenericHardwareSpecializer::new(hardware)))
  }
}

pub struct HIDDeviceImpl {
  connected: Arc<AtomicBool>,
  device_event_sender: broadcast::Sender<HardwareEvent>,
  device: Arc<Mutex<HidAsyncDevice>>,
}

impl HIDDeviceImpl {
  pub fn new(device: HidAsyncDevice) -> Self {
    let (device_event_sender, _) = broadcast::channel(256);
    Self {
      device: Arc::new(Mutex::new(device)),
      connected: Arc::new(AtomicBool::new(true)),
      device_event_sender,
    }
  }
}

impl HardwareInternal for HIDDeviceImpl {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.device_event_sender.subscribe()
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let connected = self.connected.clone();
    Box::pin(async move {
      connected.store(false, Ordering::SeqCst);
      Ok(())
    })
  }

  fn read_value(
    &self,
    _msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    unimplemented!();
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let device = self.device.clone();
    let data = msg.data.clone();
    Box::pin(async move {
      device.lock().await.write(&data).await.map_err(|e| {
        ButtplugDeviceError::DeviceCommunicationError(format!(
          "Cannot write to HID Device: {:?}.",
          e
        ))
      })?;
      Ok(())
    })
  }

  fn subscribe(
    &self,
    _msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    unimplemented!();
  }

  fn unsubscribe(
    &self,
    _msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    unimplemented!();
  }
}
