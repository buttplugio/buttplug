// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::messages::{
    ButtplugServerMessage,
    DeviceAdded,
    DeviceRemoved,
    ScanningFinished,
  },
  server::{
    device::{
      configuration::DeviceConfigurationManager,
      hardware::{
        communication::DeviceCommunicationEvent,
        ButtplugDevice,
        HardwareCreator,
        HardwareEvent
      }
    },
  },
  util::async_manager,
};
use dashmap::{DashMap, DashSet};
use futures::FutureExt;
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;
use tracing;
use tracing_futures::Instrument;

pub struct DeviceManagerEventLoop {
  device_config_manager: DeviceConfigurationManager,
  /// Maps device index (exposed to the outside world) to actual device objects held by the server.
  device_map: Arc<DashMap<u32, Arc<ButtplugDevice>>>,
  /// Broadcaster that relays device events in the form of Buttplug Messages to
  /// whoever owns the Buttplug Server.
  server_sender: broadcast::Sender<ButtplugServerMessage>,
  /// As the device manager owns the Device Communication Managers, it will have
  /// a receiver that the comm managers all send thru.
  device_comm_receiver: mpsc::Receiver<DeviceCommunicationEvent>,
  /// Sender for device events, passed to new devices when they are created.
  device_event_sender: mpsc::Sender<HardwareEvent>,
  /// Receiver for device events, which the event loops to handle events.
  device_event_receiver: mpsc::Receiver<HardwareEvent>,
  /// True if StartScanning has been called but no ScanningFinished has been
  /// emitted yet.
  scanning_in_progress: bool,
  /// Holds the status of comm manager scanning states (scanning/not scanning).
  comm_manager_scanning_statuses: Vec<Arc<AtomicBool>>,
  /// Devices currently trying to connect.
  connecting_devices: Arc<DashSet<String>>,
  /// Cancellation token for the event loop
  loop_cancellation_token: CancellationToken
}

impl DeviceManagerEventLoop {
  pub fn new(
    device_config_manager: DeviceConfigurationManager,
    device_map: Arc<DashMap<u32, Arc<ButtplugDevice>>>,
    loop_cancellation_token: CancellationToken,
    server_sender: broadcast::Sender<ButtplugServerMessage>,
    device_comm_receiver: mpsc::Receiver<DeviceCommunicationEvent>,
  ) -> Self {
    let (device_event_sender, device_event_receiver) = mpsc::channel(256);
    Self {
      device_config_manager: device_config_manager,
      server_sender,
      device_map,
      device_comm_receiver,
      device_event_sender,
      device_event_receiver,
      scanning_in_progress: false,
      comm_manager_scanning_statuses: vec![],
      connecting_devices: Arc::new(DashSet::new()),
      loop_cancellation_token
    }
  }

  fn try_create_new_device(
    &mut self,
    device_address: String,
    device_creator: Box<dyn HardwareCreator>,
  ) {
    debug!("Trying to create device at address {}", device_address);
    let device_event_sender_clone = self.device_event_sender.clone();

    // First off, we need to see if we even have a configuration available for the device we're
    // trying to create. If we don't, exit, because this isn't actually an error. However, if we
    // *do* have a configuration but something goes wrong after this, then it's an error.
    let protocol_builder = match self.device_config_manager.protocol_instance_factory(&device_creator.specifier()) {
      Some(builder) => builder,
      None => {
        debug!("Device {} not matched to protocol, ignoring.", device_address);
        return;
      }
    };

    let create_device_future =
      ButtplugDevice::try_create_device(protocol_builder, device_creator);
    let connecting_devices = self.connecting_devices.clone();

    async_manager::spawn(async move {
      match create_device_future.await {
        Ok(option_dev) => match option_dev {
          Some(device) => {
            if device_event_sender_clone
              .send(HardwareEvent::Connected(Arc::new(device)))
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
          name = tracing::field::display(name.clone()),
          address = tracing::field::display(address.clone())
        );
        let _enter = span.enter();
        info!("Device {} ({}) found.", name, address);
        // Make sure the device isn't on the deny list, or is on the allow list if anything is on it.
        if !self.device_config_manager.address_allowed(&address) {
          return;
        }
        debug!("Device {} allowed via configuration file, continuing.", address);

        // Check to make sure the device isn't already connected. If it is, drop it.
        if self
          .device_map
          .iter()
          .any(|entry| entry.value().device_impl_address() == address)
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
        self.try_create_new_device(address, creator);
      }
      DeviceCommunicationEvent::DeviceManagerAdded(status) => {
        self.comm_manager_scanning_statuses.push(status);
      }
    }
  }

  async fn handle_device_event(&mut self, device_event: HardwareEvent) {
    trace!("Got device event: {:?}", device_event);
    match device_event {
      HardwareEvent::Connected(device) => {
        let span = info_span!(
          "device registration",
          name = tracing::field::display(device.name()),
          identifier = tracing::field::debug(device.device_identifier())
        );
        let _enter = span.enter();

        // See if we have a reserved or reusable device index here.
        let device_index = self.device_config_manager.device_index(device.device_identifier());
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
      HardwareEvent::Disconnected(address) => {
        let mut device_index = None;
        for device_pair in self.device_map.iter() {
          if device_pair.value().device_identifier().address() == &address {
            device_index = Some(*device_pair.key());
            break;
          }
        }
        if let Some(device_index) = device_index {
          self
            .device_map
            .remove(&device_index)
            .expect("Remove will always work.");
          if self
            .server_sender
            .send(DeviceRemoved::new(device_index).into())
            .is_err()
          {
            debug!("Server not currently available, dropping Device Removed event.");
          }
        }
      }
      HardwareEvent::Notification(_address, _endpoint, _data) => {
        // TODO At some point here we need to fill this in for RawSubscribe and
        // other sensor subscriptions.
      }
    }
  }

  pub async fn run(&mut self) {
    loop {
      select! {
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
        _ = self.loop_cancellation_token.cancelled().fuse() => {
          info!("Device event loop cancelled, exiting.");
          return
        }
      }
    }
  }
}
