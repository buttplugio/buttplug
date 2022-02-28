use super::{
  comm_managers::DeviceCommunicationEvent,
  device_manager::DeviceUserConfig,
  ping_timer::PingTimer,
};
use crate::{
  core::messages::{
    ButtplugServerMessage,
    DeviceAdded,
    DeviceRemoved,
    ScanningFinished,
    StopDeviceCmd,
  },
  device::{
    configuration_manager::DeviceConfigurationManager,
    ButtplugDevice,
    ButtplugDeviceEvent,
    ButtplugDeviceImplCreator,
  },
  util::async_manager,
};
use dashmap::{DashMap, DashSet};
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};
use tokio::sync::{broadcast, mpsc};
use tracing;
use tracing_futures::Instrument;

pub struct DeviceManagerEventLoop {
  device_config_manager: Arc<DeviceConfigurationManager>,
  device_index_generator: u32,
  device_map: Arc<DashMap<u32, Arc<ButtplugDevice>>>,
  device_user_config: Arc<DashMap<String, DeviceUserConfig>>,
  ping_timer: Arc<PingTimer>,
  /// Maps device addresses to indexes, so they can be reused on reconnect.
  device_index_map: Arc<DashMap<String, u32>>,
  /// Broadcaster that relays device events in the form of Buttplug Messages to
  /// whoever owns the Buttplug Server.
  server_sender: broadcast::Sender<ButtplugServerMessage>,
  /// As the device manager owns the Device Communication Managers, it will have
  /// a receiver that the comm managers all send thru.
  device_comm_receiver: mpsc::Receiver<DeviceCommunicationEvent>,
  /// Sender for device events, passed to new devices when they are created.
  device_event_sender: mpsc::Sender<ButtplugDeviceEvent>,
  /// Receiver for device events, which the event loops to handle events.
  device_event_receiver: mpsc::Receiver<ButtplugDeviceEvent>,
  /// True if StartScanning has been called but no ScanningFinished has been
  /// emitted yet.
  scanning_in_progress: bool,
  /// Holds the status of comm manager scanning states (scanning/not scanning).
  comm_manager_scanning_statuses: Vec<Arc<AtomicBool>>,
  /// Devices currently trying to connect.
  connecting_devices: Arc<DashSet<String>>,
}

impl DeviceManagerEventLoop {
  pub fn new(
    device_config_manager: Arc<DeviceConfigurationManager>,
    server_sender: broadcast::Sender<ButtplugServerMessage>,
    device_map: Arc<DashMap<u32, Arc<ButtplugDevice>>>,
    device_user_config: Arc<DashMap<String, DeviceUserConfig>>,
    ping_timer: Arc<PingTimer>,
    device_comm_receiver: mpsc::Receiver<DeviceCommunicationEvent>,
  ) -> Self {
    let (device_event_sender, device_event_receiver) = mpsc::channel(256);
    Self {
      device_config_manager,
      server_sender,
      device_map,
      device_user_config,
      ping_timer,
      device_comm_receiver,
      device_index_generator: 0,
      device_index_map: Arc::new(DashMap::new()),
      device_event_sender,
      device_event_receiver,
      scanning_in_progress: false,
      comm_manager_scanning_statuses: vec![],
      connecting_devices: Arc::new(DashSet::new()),
    }
  }

  fn try_create_new_device(
    &mut self,
    device_address: String,
    device_creator: Box<dyn ButtplugDeviceImplCreator>,
  ) {
    let device_event_sender_clone = self.device_event_sender.clone();
    let create_device_future =
      ButtplugDevice::try_create_device(self.device_config_manager.clone(), device_creator);
    let device_user_config = self.device_user_config.clone();
    let connecting_devices = self.connecting_devices.clone();
    async_manager::spawn(async move {
      match create_device_future.await {
        Ok(option_dev) => match option_dev {
          Some(mut device) => {
            // The device was created, now we need to customize it before handing it to the system.
            if let Some(device_config) = device_user_config.get(device.address()) {
              if let Some(device_name) = device_config.display_name() {
                info!("Display name found for {} ({}), setting to {}", device.name(), device.address(), device_name);
                device.set_display_name(device_name);
              }
            }

            if device_event_sender_clone
              .send(ButtplugDeviceEvent::Connected(Arc::new(device)))
              .await
              .is_err() {
              error!("Device manager disappeared before connection established, device will be dropped.");
            }
          }
          None => debug!("Device could not be matched to a protocol."),
        },
        Err(e) => error!("Device errored while trying to connect: {}", e),
      }
      connecting_devices.remove(&device_address);
    }.instrument(tracing::Span::current()));
  }

  async fn handle_device_communication(&mut self, event: DeviceCommunicationEvent) {
    match event {
      DeviceCommunicationEvent::ScanningStarted => {
        self.scanning_in_progress = true;
      }
      DeviceCommunicationEvent::ScanningFinished => {
        debug!(
          "System signaled that scanning was finished, check to see if all managers are finished."
        );
        if !self.scanning_in_progress {
          debug!("Manager finished before scanning was fully started, continuing event loop.");
          return;
        }
        if self
          .comm_manager_scanning_statuses
          .iter()
          .any(|x| x.load(Ordering::SeqCst))
        {
          debug!("At least one manager still scanning, continuing event loop.");
          return;
        }
        debug!("All managers finished, emitting ScanningFinished");
        self.scanning_in_progress = false;
        if self
          .server_sender
          .send(ScanningFinished::default().into())
          .is_err()
        {
          info!("Server disappeared, exiting loop.");
        }
      }
      DeviceCommunicationEvent::DeviceFound {
        name,
        address,
        creator,
      } => {
        let span = info_span!(
          "device creation",
          name = tracing::field::display(name),
          address = tracing::field::display(address.clone())
        );
        let _enter = span.enter();

        // Check to make sure the device isn't already connected. If it is, drop it.
        if self
          .device_map
          .iter()
          .any(|entry| entry.value().address() == address)
        {
          debug!(
            "Device {} already connected, ignoring new device event.",
            address
          );
          return;
        }

        // Some device managers (like bluetooth) can send multiple DeviceFound events for the same
        // device, due to how things like advertisements work. We'll filter this at the
        // DeviceManager level to make sure that even if a badly coded DCM throws multiple found
        // events, we only listen to the first one.
        if self.connecting_devices.contains(&address) {
          info!(
            "Device {} currently trying to connect, ignoring new device event.",
            address
          );
          return;
        }

        self.connecting_devices.insert(address.clone());

        // Make sure the device isn't on the deny list
        if let Some(config) = self.device_user_config.get(&address) {
          info!(
            "Device {} has a user configuration entry, checking.",
            address
          );
          if let Some(true) = config.deny() {
            info!("Denied device address {} found, ignoring.", address);
            return;
          }
        } else {
          info!("Device {} has no user configuration entry.", address);
        }

        let mut is_allowed = true;
        {
          // Make sure allow list isn't active, or that the device is in the allow list if it is.
          let mut allow_list = self
            .device_user_config
            .iter()
            .filter(|x| *x.value().allow() == Some(true))
            .peekable();
          if allow_list.peek().is_some() {
            if !allow_list.any(|x| *x.key() == address) {
              info!(
                "Allow list active and device address {} not found, ignoring.",
                address
              );
              is_allowed = false;
            } else {
              info!("Allow list active and device address {} found.", address);
            }
          }
        }
        if !is_allowed {
          return;
        }

        self.try_create_new_device(address.clone(), creator);
        self.connecting_devices.remove(address.as_str());
      }
      DeviceCommunicationEvent::DeviceManagerAdded(status) => {
        self.comm_manager_scanning_statuses.push(status);
      }
    }
  }

  async fn handle_device_event(&mut self, device_event: ButtplugDeviceEvent) {
    trace!("Got device event: {:?}", device_event);
    match device_event {
      ButtplugDeviceEvent::Connected(device) => {
        let span = info_span!(
          "device registration",
          name = tracing::field::display(device.name()),
          address = tracing::field::display(device.address())
        );
        let _enter = span.enter();
        let generated_device_index = self.device_index_generator;
        self.device_index_generator += 1;
        // See if we have a reusable device index here.
        let device_index = if let Some(id) = self.device_index_map.get(device.address()) {
          *id.value()
        } else {
          self
            .device_index_map
            .insert(device.address().to_owned(), generated_device_index);
          generated_device_index
        };
        // Since we can now reuse device indexes, this means we might possibly
        // stomp on devices already in the map if they don't register a
        // disconnect before we try to insert the new device. If we have a
        // device already in the map with the same index (and therefore same
        // address), consider it disconnected and eject it from the map. This
        // should also trigger a disconnect event before our new DeviceAdded
        // message goes out, so timing matters here.
        if let Some((_, old_device)) = self.device_map.remove(&device_index) {
          info!("Device map contains key {}.", device_index);
          // After removing the device from the array, manually disconnect it to
          // make sure the event is thrown.
          if let Err(err) = old_device.disconnect().await {
            // If we throw an error during the disconnect, we can't really do
            // anything with it, but should at least log it.
            error!("Error during index collision disconnect: {:?}", err);
          }
        } else {
          info!("Device map does not contain key {}.", device_index);
        }

        // Create event loop for forwarding device events into our selector.
        let mut event_listener = device.event_stream();
        let event_sender = self.device_event_sender.clone();
        async_manager::spawn(async move {
          while let Ok(event) = event_listener.recv().await {
            event_sender
              .send(event)
              .await
              .expect("Should always succeed since it goes to the Device Manager which owns us.");
          }
        });

        info!("Assigning index {} to {}", device_index, device.name());
        let device_added_message =
          DeviceAdded::new(device_index, &device.name(), &device.message_attributes());
        self.device_map.insert(device_index, device);
        // After that, we can send out to the server's event listeners to let
        // them know a device has been added.
        if self
          .server_sender
          .send(device_added_message.into())
          .is_err()
        {
          debug!("Server not currently available, dropping Device Added event.");
        }
      }
      ButtplugDeviceEvent::Removed(address) => {
        let device_index = *self
          .device_index_map
          .get(&address)
          .expect("Index must exist to get here.")
          .value();
        self.device_map.remove(&device_index);
        if self
          .server_sender
          .send(DeviceRemoved::new(device_index).into())
          .is_err()
        {
          debug!("Server not currently available, dropping Device Removed event.");
        }
      }
      ButtplugDeviceEvent::Notification(_address, _endpoint, _data) => {
        // TODO At some point here we need to fill this in for RawSubscribe and
        // other sensor subscriptions.
      }
    }
  }

  async fn handle_ping_timeout(&self) {
    error!("Pinged out, stopping devices");
    let mut fut_vec = FuturesUnordered::new();
    self.device_map.iter().for_each(|dev| {
      let device = dev.value();
      fut_vec.push(device.parse_message(StopDeviceCmd::new(1).into()))
    });
    async_manager::spawn(async move {
      while let Some(val) = fut_vec.next().await {
        // Device index doesn't matter here, since we're sending the
        // message directly to the device itself.
        if let Err(e) = val {
          error!("Error stopping device on ping timeout: {}", e);
        }
      }
    });
  }

  pub async fn run(&mut self) {
    loop {
      select! {
        // If we have a ping timeout, stop all devices
        _ = self.ping_timer.ping_timeout_waiter().fuse() => {
          self.handle_ping_timeout().await;
        },
        device_comm_msg = self.device_comm_receiver.recv().fuse() => {
          if let Some(msg) = device_comm_msg {
            self.handle_device_communication(msg).await;
          } else {
            break;
          }
        }
        device_event_msg = self.device_event_receiver.recv().fuse() => {
          if let Some(msg) = device_event_msg {
            self.handle_device_event(msg).await;
          } else {
            panic!("We shouldn't be able to get here since we also own the sender.");
          }
        },
      }
    }
  }
}
