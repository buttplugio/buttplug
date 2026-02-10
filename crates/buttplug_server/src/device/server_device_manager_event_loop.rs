// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::{
  message::{ButtplugServerMessageV4, DeviceListV4, ScanningFinishedV0},
  util::async_manager,
};
use buttplug_server_device_config::DeviceConfigurationManager;
use tracing::info_span;

use crate::device::{
  DeviceHandle,
  InternalDeviceEvent,
  device_handle::build_device_handle,
  hardware::communication::{HardwareCommunicationManager, HardwareCommunicationManagerEvent},
  protocol::ProtocolManager,
};
use dashmap::{DashMap, DashSet};
use futures::{FutureExt, future};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;
use tracing_futures::Instrument;

use super::server_device_manager::DeviceManagerCommand;

/// Scanning state machine for the device manager event loop.
/// Replaces the previous combination of scanning_bringup_in_progress, scanning_started,
/// and stop_scanning_received fields with explicit states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ScanningState {
  /// No scanning activity. This is the initial state.
  #[default]
  Idle,
  /// StartScanning called, waiting for all comm managers to start.
  /// ScanningFinished events are ignored in this state.
  BringupInProgress,
  /// Actively scanning. Will emit ScanningFinished when all managers finish.
  Active,
  /// StopScanning was called. Will NOT emit ScanningFinished.
  ActiveStopRequested,
}

pub(super) struct ServerDeviceManagerEventLoop {
  comm_managers: Vec<Box<dyn HardwareCommunicationManager>>,
  device_config_manager: Arc<DeviceConfigurationManager>,
  device_command_receiver: mpsc::Receiver<DeviceManagerCommand>,
  /// Maps device index (exposed to the outside world) to device handles held by the server.
  device_map: Arc<DashMap<u32, DeviceHandle>>,
  /// Broadcaster that relays device events in the form of Buttplug Messages to
  /// whoever owns the Buttplug Server.
  server_sender: broadcast::Sender<ButtplugServerMessageV4>,
  /// As the device manager owns the Device Communication Managers, it will have
  /// a receiver that the comm managers all send thru.
  device_comm_receiver: mpsc::Receiver<HardwareCommunicationManagerEvent>,
  /// Sender for device events, passed to new devices when they are created.
  device_event_sender: mpsc::Sender<InternalDeviceEvent>,
  /// Receiver for device events, which the event loops to handle events.
  device_event_receiver: mpsc::Receiver<InternalDeviceEvent>,
  /// Current scanning state machine state.
  scanning_state: ScanningState,
  /// Devices currently trying to connect.
  connecting_devices: Arc<DashSet<String>>,
  /// Cancellation token for the event loop
  loop_cancellation_token: CancellationToken,
  /// Protocol map, for mapping user definitions to protocols
  protocol_manager: ProtocolManager,
}

impl ServerDeviceManagerEventLoop {
  pub fn new(
    comm_managers: Vec<Box<dyn HardwareCommunicationManager>>,
    device_config_manager: Arc<DeviceConfigurationManager>,
    device_map: Arc<DashMap<u32, DeviceHandle>>,
    loop_cancellation_token: CancellationToken,
    server_sender: broadcast::Sender<ButtplugServerMessageV4>,
    device_comm_receiver: mpsc::Receiver<HardwareCommunicationManagerEvent>,
    device_command_receiver: mpsc::Receiver<DeviceManagerCommand>,
  ) -> Self {
    let (device_event_sender, device_event_receiver) = mpsc::channel(256);
    Self {
      comm_managers,
      device_config_manager,
      server_sender,
      device_map,
      device_comm_receiver,
      device_event_sender,
      device_event_receiver,
      device_command_receiver,
      scanning_state: ScanningState::Idle,
      connecting_devices: Arc::new(DashSet::new()),
      loop_cancellation_token,
      protocol_manager: ProtocolManager::default(),
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
    // Only start from Idle state
    if self.scanning_state != ScanningState::Idle {
      debug!(
        "System already scanning (state: {:?}), ignoring new scanning request",
        self.scanning_state
      );
      return;
    }

    // Also check if hardware is still scanning (edge case: state is Idle but hardware lagging)
    if self.scanning_status() {
      debug!("Hardware still scanning, ignoring new scanning request");
      return;
    }

    info!("No scan currently in progress, starting new scan.");
    self.scanning_state = ScanningState::BringupInProgress;

    let fut_vec: Vec<_> = self
      .comm_managers
      .iter_mut()
      .map(|guard| guard.start_scanning())
      .collect();
    // TODO If start_scanning fails anywhere, this will ignore it. We should maybe at least log?
    future::join_all(fut_vec).await;

    debug!("Scanning started for all hardware comm managers.");
    // Check if stop was requested during bringup
    if self.scanning_state == ScanningState::ActiveStopRequested {
      debug!("Stop was requested during bringup, staying in ActiveStopRequested");
    } else {
      self.scanning_state = ScanningState::Active;
    }
  }

  async fn handle_stop_scanning(&mut self) {
    // Transition to stop-requested state (only meaningful if currently scanning)
    match self.scanning_state {
      ScanningState::Active => {
        self.scanning_state = ScanningState::ActiveStopRequested;
      }
      ScanningState::BringupInProgress => {
        // Edge case: stop requested during bringup.
        // The bringup completion in handle_start_scanning will see this and not transition to Active.
        self.scanning_state = ScanningState::ActiveStopRequested;
      }
      _ => {
        debug!(
          "Stop scanning called in state {:?}, no state change needed",
          self.scanning_state
        );
      }
    }

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
          "System signaled that scanning was finished, state: {:?}",
          self.scanning_state
        );

        match self.scanning_state {
          ScanningState::Idle => {
            // Spurious event, ignore
            debug!("Received ScanningFinished in Idle state, ignoring");
          }
          ScanningState::BringupInProgress => {
            // Comm manager finished before we completed bringup - ignore for now
            debug!("Hardware Comm Manager finished before scanning was fully started, ignoring");
          }
          ScanningState::Active => {
            // Check if all hardware has actually stopped
            if !self.scanning_status() {
              debug!("All managers finished, emitting ScanningFinished");
              self.scanning_state = ScanningState::Idle;
              if self
                .server_sender
                .send(ScanningFinishedV0::default().into())
                .is_err()
              {
                info!("Server disappeared, exiting loop.");
              }
            }
          }
          ScanningState::ActiveStopRequested => {
            // Stop was requested, don't emit ScanningFinished
            if !self.scanning_status() {
              debug!("All managers finished after stop request, not emitting ScanningFinished");
              self.scanning_state = ScanningState::Idle;
            }
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
        let protocol_specializers = self.protocol_manager.protocol_specializers(
          &creator.specifier(),
          self.device_config_manager.base_communication_specifiers(),
          self.device_config_manager.user_communication_specifiers(),
        );

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

        // Clone sender again for the forwarding task that build_device_handle will spawn
        let device_event_sender_for_forwarding = self.device_event_sender.clone();

        async_manager::spawn(async move {
          match build_device_handle(
            device_config_manager,
            creator,
            protocol_specializers,
            device_event_sender_for_forwarding,
          ).await {
            Ok(device_handle) => {
              if device_event_sender_clone
                .send(InternalDeviceEvent::Connected(device_handle))
                .await
                .is_err() {
                error!("Device manager disappeared before connection established, device will be dropped.");
              }
            },
            Err(e) => {
              error!("Device errored while trying to connect: {:?}", e);
            }
          }
          connecting_devices.remove(&address);
        }.instrument(span));
      }
    }
  }

  fn generate_device_list(&self) -> DeviceListV4 {
    let devices = self
      .device_map
      .iter()
      .map(|device| device.value().as_device_message_info(*device.key()))
      .collect();
    DeviceListV4::new(devices)
  }

  async fn handle_device_event(&mut self, device_event: InternalDeviceEvent) {
    trace!("Got device event: {:?}", device_event);
    match device_event {
      InternalDeviceEvent::Connected(device_handle) => {
        let span = info_span!(
          "device registration",
          name = tracing::field::display(device_handle.name()),
          identifier = tracing::field::debug(device_handle.identifier())
        );
        let _enter = span.enter();

        // Get the index from the device
        let device_index = device_handle.definition().index();
        // Since we can now reuse device indexes, this means we might possibly
        // stomp on devices already in the map if they don't register a
        // disconnect before we try to insert the new device. If we have a
        // device already in the map with the same index (and therefore same
        // address), consider it disconnected and eject it from the map. This
        // should also trigger a disconnect event before our new DeviceAdded
        // message goes out, so timing matters here.
        match self.device_map.remove(&device_index) {
          Some((_, old_device)) => {
            info!("Device map contains key {}.", device_index);
            // After removing the device from the array, manually disconnect it to
            // make sure the event is thrown.
            if let Err(err) = old_device.disconnect().await {
              // If we throw an error during the disconnect, we can't really do
              // anything with it, but should at least log it.
              error!("Error during index collision disconnect: {:?}", err);
            }
          }
          _ => {
            info!("Device map does not contain key {}.", device_index);
          }
        }

        // Note: The device event forwarding task is now spawned in build_device_handle(),
        // so we no longer need to create it here.

        info!(
          "Assigning index {} to {}",
          device_index,
          device_handle.name()
        );
        self.device_map.insert(device_index, device_handle.clone());

        let device_update_message: ButtplugServerMessageV4 = self.generate_device_list().into();

        // After that, we can send out to the server's event listeners to let
        // them know a device has been added.
        if self.server_sender.send(device_update_message).is_err() {
          debug!("Server not currently available, dropping Device Added event.");
        }
      }
      InternalDeviceEvent::Disconnected(identifier) => {
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
          let device_update_message: ButtplugServerMessageV4 = self.generate_device_list().into();
          if self.server_sender.send(device_update_message).is_err() {
            debug!("Server not currently available, dropping Device Removed event.");
          }
        }
      }
      InternalDeviceEvent::Notification(_, message) => {
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
