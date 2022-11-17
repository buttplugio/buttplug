use super::usb_hardware::{DeviceExt, UsbHardwareConnector};
use crate::{
  core::errors::ButtplugDeviceError,
  core::ButtplugResultFuture,
  server::device::hardware::communication::{
    HardwareCommunicationManager,
    HardwareCommunicationManagerBuilder,
    HardwareCommunicationManagerEvent,
    HardwareSpecificError,
    TimedRetryCommunicationManager,
    TimedRetryCommunicationManagerImpl,
  },
  util::async_manager,
};
use async_trait::async_trait;
use futures_util::FutureExt;
use rusb::{self, UsbContext};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::{broadcast, mpsc};

#[derive(Default, Clone)]
pub struct UsbCommunicationManagerBuilder {}

impl HardwareCommunicationManagerBuilder for UsbCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: mpsc::Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    let comm_manager = UsbCommunicationManager::new(sender);
    if rusb::has_hotplug() {
      Box::new(comm_manager)
    } else {
      Box::new(TimedRetryCommunicationManager::new(comm_manager))
    }
  }
}

pub struct UsbCommunicationManager {
  sender: mpsc::Sender<HardwareCommunicationManagerEvent>,
  context: rusb::Context,
  scanning_status: Arc<AtomicBool>,
  hotplug_registration: Arc<Mutex<Option<rusb::Registration<rusb::Context>>>>,
}

impl UsbCommunicationManager {
  fn new(sender: mpsc::Sender<HardwareCommunicationManagerEvent>) -> Self {
    Self {
      sender,
      context: rusb::Context::new().expect("USB manager couldn't create libusb context."),
      scanning_status: Arc::new(AtomicBool::new(false)),
      hotplug_registration: Arc::new(Mutex::new(None)),
    }
  }
}

#[async_trait]
impl HardwareCommunicationManager for UsbCommunicationManager {
  fn name(&self) -> &'static str {
    "UsbCommunicationManager"
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    trace!("USB manager starting scan");
    self.scanning_status.store(true, Ordering::SeqCst);
    let context = self.context.clone();
    let scanning_status = self.scanning_status.clone();
    let comm_sender = self.sender.clone();
    let hotplug_registration = self.hotplug_registration.clone();
    async move {
      trace!("USB manager registering hotplugger");
      let (hotplug_sender, hotplug_receiver) = broadcast::channel(256);
      let hotplugger = UsbHotplugger {
        sender: hotplug_sender,
      };
      async_manager::spawn(handle_usb_context_events(context.clone(), scanning_status));
      async_manager::spawn(handle_usb_hotplug_events(hotplug_receiver, comm_sender));
      let registration = rusb::HotplugBuilder::new()
        .enumerate(true)
        .register(context, Box::new(hotplugger))
        .map_err(|e: rusb::Error| {
          ButtplugDeviceError::from(HardwareSpecificError::UsbError(format!(
            "USB manager couldn't register hotplugger: {e:?}"
          )))
        })?;
      let mut lock = hotplug_registration.lock().await;
      *lock = Some(registration);
      Ok(())
    }
    .boxed()
  }

  fn stop_scanning(&mut self) -> ButtplugResultFuture {
    trace!("USB manager stopping scan");
    self.scanning_status.store(false, Ordering::SeqCst);
    let hotplug_registration = self.hotplug_registration.clone();
    async move {
      trace!("USB manager de-registering hotplugger");
      let mut lock = hotplug_registration.lock().await;
      *lock = None;
      Ok(())
    }
    .boxed()
  }

  fn can_scan(&self) -> bool {
    self.scanning_status.load(Ordering::SeqCst)
  }
}

#[derive(Debug, Clone)]
pub enum UsbHotplugEvent {
  Arrived(rusb::Device<rusb::Context>),
  Left(rusb::Device<rusb::Context>),
}

/// Thread that handles libusb context events during scan.
async fn handle_usb_context_events(context: rusb::Context, scanning_status: Arc<AtomicBool>) {
  loop {
    if !scanning_status.load(Ordering::SeqCst) {
      debug!("USB manager ended scan");
      return;
    }
    if let Err(e) = context.handle_events(Some(Duration::from_millis(500))) {
      error!("libusb context handle_events failed: {e:?}");
      return;
    }
  }
}

/// Thread that handles device arrived events from hotplugger during scan.
async fn handle_usb_hotplug_events(
  mut hotplug_receiver: broadcast::Receiver<UsbHotplugEvent>,
  comm_sender: mpsc::Sender<HardwareCommunicationManagerEvent>,
) {
  while let Ok(event) = hotplug_receiver.recv().await {
    if let UsbHotplugEvent::Arrived(device) = event {
      let name = device.name();
      let address = device.qualified_address();
      debug!("USB manager found device {name}, {address}");
      let creator = Box::new(UsbHardwareConnector::new(
        device,
        Some(hotplug_receiver.resubscribe()),
      ));
      let event = HardwareCommunicationManagerEvent::DeviceFound {
        name,
        address,
        creator,
      };
      if let Err(e) = comm_sender.send(event).await {
        error!("Error sending device found message from USB manager: {e:?}");
      }
    }
  }
  debug!("USB hotplugger closing down");
}

/// Registered with an `rusb::Context` to handle hotplug events.
struct UsbHotplugger {
  sender: broadcast::Sender<UsbHotplugEvent>,
}

impl rusb::Hotplug<rusb::Context> for UsbHotplugger {
  fn device_arrived(&mut self, device: rusb::Device<rusb::Context>) {
    debug!(
      "USB hotplugger device arrived: {}",
      device.qualified_address()
    );
    if let Err(e) = self.sender.send(UsbHotplugEvent::Arrived(device)) {
      error!("USB hotplugger couldn't send device arrived hotplug event: {e:?}");
    }
  }

  fn device_left(&mut self, device: rusb::Device<rusb::Context>) {
    debug!("USB hotplugger device left: {}", device.qualified_address());
    if let Err(e) = self.sender.send(UsbHotplugEvent::Left(device)) {
      error!("USB hotplugger couldn't send device left hotplug event: {e:?}");
    }
  }
}

/// Intended as fallback if libusb doesn't have hotplug support (as is the case on Windows).
/// In this mode we will not check for device disconnection.
#[async_trait]
impl TimedRetryCommunicationManagerImpl for UsbCommunicationManager {
  fn name(&self) -> &'static str {
    "UsbCommunicationManager"
  }

  fn can_scan(&self) -> bool {
    true
  }

  async fn scan(&self) -> Result<(), ButtplugDeviceError> {
    trace!("USB manager scanning for devices");
    let devices = self
      .context
      .devices()
      .map_err(|e: rusb::Error| {
        ButtplugDeviceError::from(HardwareSpecificError::UsbError(format!("{:?}", e)))
      })?
      .iter()
      .collect::<Vec<_>>();
    for device in devices {
      let name = device.name();
      let address = device.qualified_address();
      debug!("USB manager found device {name}, {address}");
      let creator = Box::new(UsbHardwareConnector::new(device, None));
      if let Err(e) = self
        .sender
        .send(HardwareCommunicationManagerEvent::DeviceFound {
          name,
          address,
          creator,
        })
        .await
      {
        error!("Error sending device found message from USB manager: {e:?}");
        break;
      }
    }
    Ok(())
  }
}
