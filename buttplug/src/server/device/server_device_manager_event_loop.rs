// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::message::{ButtplugServerMessage, DeviceAdded, DeviceRemoved, ScanningFinished},
  server::device::{
    configuration::DeviceConfigurationManager,
    hardware::communication::{HardwareCommunicationManager, HardwareCommunicationManagerEvent},
    server_device::build_server_device,
    ServerDevice,
    ServerDeviceEvent,
  },
  util::async_manager,
};
use dashmap::{DashMap, DashSet};
use futures::{future, FutureExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;
use tracing;
use tracing_futures::Instrument;

use super::server_device_manager::DeviceManagerCommand;

pub(super) struct ServerDeviceManagerEventLoop {
  comm_managers: Vec<Box<dyn HardwareCommunicationManager>>,
  device_config_manager: Arc<DeviceConfigurationManager>,
  device_command_receiver: mpsc::Receiver<DeviceManagerCommand>,
  /// Maps device index (exposed to the outside world) to actual device objects held by the server.
  device_map: Arc<DashMap<u32, Arc<ServerDevice>>>,
  /// Broadcaster that relays device events in the form of Buttplug Messages to
  /// whoever owns the Buttplug Server.
  server_sender: broadcast::Sender<ButtplugServerMessage>,
  /// As the device manager owns the Device Communication Managers, it will have
  /// a receiver that the comm managers all send thru.
  device_comm_receiver: mpsc::Receiver<HardwareCommunicationManagerEvent>,
  /// Sender for device events, passed to new devices when they are created.
  device_event_sender: mpsc::Sender<ServerDeviceEvent>,
  /// Receiver for device events, which the event loops to handle events.
  device_event_receiver: mpsc::Receiver<ServerDeviceEvent>,
  /// True if StartScanning has been called but no ScanningFinished has been
  /// emitted yet.
  scanning_bringup_in_progress: bool,
  /// Denote whether scanning has been started since we last sent a ScanningFinished message.
  scanning_started: bool,
  /// Devices currently trying to connect.
  connecting_devices: Arc<DashSet<String>>,
  /// Cancellation token for the event loop
  loop_cancellation_token: CancellationToken,
}

impl ServerDeviceManagerEventLoop {
  pub fn new(
    comm_managers: Vec<Box<dyn HardwareCommunicationManager>>,
    device_config_manager: DeviceConfigurationManager,
    device_map: Arc<DashMap<u32, Arc<ServerDevice>>>,
    loop_cancellation_token: CancellationToken,
    server_sender: broadcast::Sender<ButtplugServerMessage>,
    device_comm_receiver: mpsc::Receiver<HardwareCommunicationManagerEvent>,
    device_command_receiver: mpsc::Receiver<DeviceManagerCommand>,
  ) -> Self {
    let (device_event_sender, device_event_receiver) = mpsc::channel(256);
    Self {
      comm_managers,
      device_config_manager: Arc::new(device_config_manager),
      server_sender,
      device_map,
      device_comm_receiver,
      device_event_sender,
      device_event_receiver,
      device_command_receiver,
      scanning_bringup_in_progress: false,
      scanning_started: false,
      connecting_devices: Arc::new(DashSet::new()),
      loop_cancellation_token,
    }
  }

  fn scanning_status(&self) -> bool {
    if self.comm_managers.iter().any(|x| x.scanning_status()) {
      debug!("At least one manager still scanning, continuing event loop.");
      return true;
    }
    false
  }

  async fn handle_start_scanning(&mut self) {
    if self.scanning_status() || self.scanning_bringup_in_progress {
      debug!("System already scanning, ignoring new scanning request");
      return;
    }

    info!("No scan currently in progress, starting new scan.");
    self.scanning_bringup_in_progress = true;
    self.scanning_started = true;
    let fut_vec: Vec<_> = self
      .comm_managers
      .iter_mut()
      .map(|guard| guard.start_scanning())
      .collect();
    // TODO If start_scanning fails anywhere, this will ignore it. We should maybe at least log?
    future::join_all(fut_vec).await;
    debug!("Scanning started for all hardware comm managers.");
    self.scanning_bringup_in_progress = false;
  }

  async fn handle_stop_scanning(&mut self) {
    let fut_vec: Vec<_> = self
      .comm_managers
      .iter_mut()
      .map(|guard| guard.stop_scanning())
      .collect();
    // TODO If stop_scanning fails anywhere, this will ignore it. We should maybe at least log?
    future::join_all(fut_vec).await;
  }

  async fn handle_device_communication(&mut self, event: HardwareCommunicationManagerEvent) {
    match event {
      HardwareCommunicationManagerEvent::ScanningFinished => {
        debug!(
          "System signaled that scanning was finished, check to see if all managers are finished."
        );
        if self.scanning_bringup_in_progress {
          debug!("Hardware Comm Manager finished before scanning was fully started, continuing event loop.");
          return;
        }
        if !self.scanning_status() && self.scanning_started {
          debug!("All managers finished, emitting ScanningFinished");
          self.scanning_started = false;
          if self
            .server_sender
            .send(ScanningFinished::default().into())
            .is_err()
          {
            info!("Server disappeared, exiting loop.");
          }
        }
      }
      HardwareCommunicationManagerEvent::DeviceFound {
        name,
        address,
        creator,
      } => {
        info!("Device {} ({}) found.", name, address);
        // Make sure the device isn't on the deny list, or is on the allow list if anything is on it.
        if !self.device_config_manager.address_allowed(&address) {
          return;
        }
        debug!(
          "Device {} allowed via configuration file, continuing.",
          address
        );

        // Check to make sure the device isn't already connected. If it is, drop what we've been
        // sent and return.
        if self
          .device_map
          .iter()
          .any(|entry| *entry.value().identifier().address() == address)
        {
          debug!(
            "Device {} already connected, ignoring new device event.",
            address
          );
          return;
        }

        // First off, we need to see if we even have a configuration available for the device we're
        // trying to create. If we don't, exit, because this isn't actually an error. However, if we
        // actually *do* have a configuration but something goes wrong after this, then it's an
        // error.
        //
        // We used to do this in build_server_device, but we shouldn't mark devices as actually
        // connecting until after this happens, so we're moving it back here.
        let protocol_specializers = self
          .device_config_manager
          .protocol_specializers(&creator.specifier());

        // If we have no identifiers, then there's nothing to do here. Throw an error.
        if protocol_specializers.is_empty() {
          debug!(
            "{}",
            format!(
              "No viable protocols for hardware {:?}, ignoring.",
              creator.specifier()
            )
          );
          return;
        }

        // Some device managers (like bluetooth) can send multiple DeviceFound events for the same
        // device, due to how things like advertisements work. We'll filter this at the
        // DeviceManager level to make sure that even if a badly coded DCM throws multiple found
        // events, we only listen to the first one.
        if !self.connecting_devices.insert(address.clone()) {
          info!(
            "Device {} currently trying to connect, ignoring new device event.",
            address
          );
          return;
        }

        let device_event_sender_clone = self.device_event_sender.clone();

        let device_config_manager = self.device_config_manager.clone();
        let connecting_devices = self.connecting_devices.clone();
        let span = info_span!(
          "device creation",
          name = tracing::field::display(name),
          address = tracing::field::display(address.clone())
        );

        async_manager::spawn(async move {
          match build_server_device(device_config_manager, creator, protocol_specializers).await {
            Ok(device) => {
              if device_event_sender_clone
                .send(ServerDeviceEvent::Connected(Arc::new(device)))
                .await
                .is_err() {
                error!("Device manager disappeared before connection established, device will be dropped.");
              }
            },
            Err(e) => {
              error!("Device errored while trying to connect: {}", e);
            }
          }
          connecting_devices.remove(&address);
        }.instrument(span));
      }
    }
  }

  async fn handle_device_event(&mut self, device_event: ServerDeviceEvent) {
    trace!("Got device event: {:?}", device_event);
    match device_event {
      ServerDeviceEvent::Connected(device) => {
        let span = info_span!(
          "device registration",
          name = tracing::field::display(device.name()),
          identifier = tracing::field::debug(device.identifier())
        );
        let _enter = span.enter();

        // See if we have a reserved or reusable device index here.
        let device_index = self.device_config_manager.device_index(device.identifier());
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
        let event_listener = device.event_stream();
        let event_sender = self.device_event_sender.clone();
        async_manager::spawn(async move {
          pin_mut!(event_listener);
          // This can fail if the event_sender loses the server before this loop dies.
          while let Some(event) = event_listener.next().await {
            if event_sender.send(event).await.is_err() {
              info!("Event sending failure in servier device manager event loop, exiting.");
              break;
            }
          }
        });

        info!("Assigning index {} to {}", device_index, device.name());
        let device_added_message = DeviceAdded::new(
          device_index,
          &device.name(),
          &device.display_name(),
          &None,
          &device.message_attributes().into(),
        );
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
      ServerDeviceEvent::Disconnected(identifier) => {
        let mut device_index = None;
        for device_pair in self.device_map.iter() {
          if *device_pair.value().identifier() == identifier {
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
      ServerDeviceEvent::Notification(_, message) => {
        if self.server_sender.send(message.into()).is_err() {
          debug!("Server not currently available, dropping Device Added event.");
        }
      }
    }
  }

  pub async fn run(&mut self) {
    debug!("Starting Device Manager Loop");
    loop {
      tokio::select! {
        device_comm_msg = self.device_comm_receiver.recv() => {
          if let Some(msg) = device_comm_msg {
            trace!("Got device communication message {:?}", msg);
            self.handle_device_communication(msg).await;
          } else {
            break;
          }
        }
        device_event_msg = self.device_event_receiver.recv() => {
          if let Some(msg) = device_event_msg {
            trace!("Got device event message {:?}", msg);
            self.handle_device_event(msg).await;
          } else {
            error!("We shouldn't be able to get here since we also own the sender.");
            break;
          }
        },
        device_command_msg = self.device_command_receiver.recv() => {
          if let Some(msg) = device_command_msg {
            trace!("Got device command message {:?}", msg);
            match msg {
              DeviceManagerCommand::StartScanning => self.handle_start_scanning().await,
              DeviceManagerCommand::StopScanning => self.handle_stop_scanning().await,
            }
          } else {
            debug!("Channel to Device Manager frontend dropped, exiting event loop.");
            break;
          }
        }
        _ = self.loop_cancellation_token.cancelled().fuse() => {
          debug!("Device event loop cancelled, exiting.");
          break;
        }
      }
    }
    debug!("Exiting Device Manager Loop");
  }
}
