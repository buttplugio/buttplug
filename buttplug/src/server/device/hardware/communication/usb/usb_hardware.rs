use super::usb_comm_manager::UsbHotplugEvent;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::hardware::communication::HardwareSpecificError,
  server::device::{
    configuration::{ProtocolCommunicationSpecifier, USBSpecifier},
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
use rusb;
use std::sync::Arc;
use std::{fmt::Debug, time::Duration};
use tokio::sync::{broadcast, Mutex};

pub trait DeviceExt {
  /// String for device bus and address on that bus.
  fn qualified_address(&self) -> String;

  /// Open device and read strings to build a display name.
  /// Never fails but may return a placeholder.
  fn name(&self) -> String;
}

impl<T: rusb::UsbContext> DeviceExt for rusb::Device<T> {
  fn qualified_address(&self) -> String {
    let bus_number = self.bus_number();
    let bus_address = self.address();
    format!("bus {bus_number}, address {bus_address}")
  }

  fn name(&self) -> String {
    let unknown = "???".to_string();
    let mut manufacturer = unknown.clone();
    let mut product = unknown.clone();
    let mut serial_number: Option<String> = None;
    if let Ok(handle) = self.open() {
      if let Ok(device_descriptor) = self.device_descriptor() {
        if let Ok(string) = handle.read_manufacturer_string_ascii(&device_descriptor) {
          manufacturer = string.trim().into();
        }
        if let Ok(string) = handle.read_product_string_ascii(&device_descriptor) {
          product = string.trim().into();
        }
        // Many devices don't have a serial number, so this will often be empty.
        if let Ok(string) = handle.read_serial_number_string_ascii(&device_descriptor) {
          serial_number = Some(string.trim().into());
        }
      };
    };
    if let Some(serial_number) = serial_number {
      format!("{manufacturer} {product} {serial_number}")
    } else {
      format!("{manufacturer} {product}")
    }
  }
}

#[derive(Debug)]
pub struct UsbHardwareConnector {
  device: rusb::Device<rusb::Context>,
  hotplug_receiver: Option<broadcast::Receiver<UsbHotplugEvent>>,
}

impl UsbHardwareConnector {
  pub fn new(
    device: rusb::Device<rusb::Context>,
    hotplug_receiver: Option<broadcast::Receiver<UsbHotplugEvent>>,
  ) -> Self {
    Self {
      device,
      hotplug_receiver,
    }
  }
}

#[async_trait]
impl HardwareConnector for UsbHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    let device_descriptor = self
      .device
      .device_descriptor()
      .expect("USB connector couldn't get device descriptor");
    ProtocolCommunicationSpecifier::USB(USBSpecifier::new_from_ids(
      device_descriptor.vendor_id(),
      device_descriptor.product_id(),
    ))
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    let name = self.device.name();
    let address = self.device.qualified_address();
    debug!("USB connector emitting a new USB device impl: {name}, {address}");
    // If libusb doesn't have hotplug support, it's normal for this to be None.
    let hotplug_receiver = self.hotplug_receiver.take();
    if hotplug_receiver.is_none() && rusb::has_hotplug() {
      // Otherwise, it's because this connect method has been called multiple times,
      // but we gave the hotplug receiver to the hardware struct on by the first call.
      warn!("USB hardware connectors shouldn't be reused. Hotplug detection disabled.");
    }
    let hardware_internal = UsbHardware::try_create(self.device.clone(), hotplug_receiver)?;
    let hardware = Hardware::new(
      &name,
      &address,
      &[Endpoint::TxVendorControl],
      Box::new(hardware_internal),
    );
    Ok(Box::new(GenericHardwareSpecializer::new(hardware)))
  }
}

#[derive(Clone, Debug)]
pub struct UsbHardware {
  handle: Arc<Mutex<rusb::DeviceHandle<rusb::Context>>>,
  address: String,
  event_sender: broadcast::Sender<HardwareEvent>,
}

impl UsbHardware {
  pub fn try_create(
    device: rusb::Device<rusb::Context>,
    hotplug_receiver: Option<broadcast::Receiver<UsbHotplugEvent>>,
  ) -> Result<Self, ButtplugDeviceError> {
    let handle = device.open().map_err(|e: rusb::Error| {
      ButtplugDeviceError::from(HardwareSpecificError::UsbError(format!("{:?}", e)))
    })?;
    let (device_event_sender, _) = broadcast::channel(256);
    let address = device.qualified_address();

    if let Some(hotplug_receiver) = hotplug_receiver {
      async_manager::spawn(handle_usb_hotplug_events(
        hotplug_receiver,
        device_event_sender.clone(),
        address.clone(),
      ));
    }

    Ok(Self {
      handle: Arc::new(Mutex::new(handle)),
      address,
      event_sender: device_event_sender,
    })
  }
}

/// This is based on the Buttplug C# code for driving the Rez Trance Vibrator,
/// the only supported non-HID USB device in that previous version of Buttplug,
/// and will need to be extended to support with any other USB device.
/// See <https://github.com/buttplugio/buttplug-csharp/blob/master/Buttplug.Server.Managers.WinUSBManager/WinUSBDeviceImpl.cs>.
impl HardwareInternal for UsbHardware {
  /// We shouldn't have to do anything, assuming the `UsbHardware` gets dropped sometime after this,
  /// which should close the `rusb::DeviceHandle` and its underlying libusb handle.
  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Ok(())).boxed()
  }

  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.event_sender.subscribe()
  }

  fn read_value(
    &self,
    _msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "USB hardware does not support read_value".to_owned(),
    )))
    .boxed()
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    if msg.endpoint != Endpoint::TxVendorControl {
      return future::ready(Err(ButtplugDeviceError::InvalidEndpoint(msg.endpoint))).boxed();
    }

    let handle = self.handle.clone();
    let data = msg.data.clone();
    let address = self.address.clone();
    let event_sender = self.event_sender.clone();
    async move {
      let lock = handle.lock().await;
      let result = lock.write_control(
        rusb::request_type(
          rusb::Direction::Out,
          rusb::RequestType::Vendor,
          rusb::Recipient::Interface,
        ),
        1,
        data[0] as u16,
        0,
        &[],
        Duration::from_millis(100),
      );
      if result == Err(rusb::Error::NoDevice) {
        if let Err(e) = event_sender.send(HardwareEvent::Disconnected(address)) {
          error!("USB hardware failed to send disconnected event: {e:?}");
        }
      }
      result.map_err(|e: rusb::Error| {
        ButtplugDeviceError::from(HardwareSpecificError::UsbError(format!("{e:?}")))
      })?;
      Ok(())
    }
    .boxed()
  }

  fn subscribe(
    &self,
    _msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "USB hardware does not support subscribe".to_owned(),
    )))
    .boxed()
  }

  fn unsubscribe(
    &self,
    _msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "USB hardware does not support unsubscribe".to_owned(),
    )))
    .boxed()
  }
}

/// Handle any disconnection events relevant to this device during scan.
async fn handle_usb_hotplug_events(
  mut hotplug_receiver: broadcast::Receiver<UsbHotplugEvent>,
  event_sender: broadcast::Sender<HardwareEvent>,
  address: String,
) {
  while let Ok(event) = hotplug_receiver.recv().await {
    if let UsbHotplugEvent::Left(device) = event {
      if device.qualified_address() == address {
        if let Err(e) = event_sender.send(HardwareEvent::Disconnected(address.clone())) {
          error!("USB hardware hotplug handler failed to send disconnected event: {e:?}");
        }
      }
    }
  }
  debug!("USB hardware hotplug handler for {address} closing down");
}
