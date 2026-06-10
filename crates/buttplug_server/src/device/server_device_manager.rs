// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug Device Manager, manages Device Subtype (Platform/Communication bus
//! specific) Managers

use crate::{
  ButtplugServerError,
  ButtplugServerResultFuture,
  device::{
    DeviceHandle,
    OutputObservation,
    hardware::communication::{HardwareCommunicationManager, HardwareCommunicationManagerBuilder},
    server_device_manager_event_loop::ServerDeviceManagerEventLoop,
  },
  message::{
    server_device_attributes::ServerDeviceAttributes,
    spec_enums::{
      ButtplugCheckedClientMessageV4,
      ButtplugDeviceCommandMessageUnionV4,
      ButtplugDeviceManagerMessageUnion,
    },
  },
};
use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugMessageError, ButtplugUnknownError},
  message::{
    self,
    ButtplugDeviceMessage,
    ButtplugMessage,
    ButtplugServerMessageV4,
    DeviceListV4,
    StopCmdV4,
  },
  util::{stream::convert_broadcast_receiver_to_stream, task::TaskScope},
};
use buttplug_server_device_config::{DeviceConfigurationManager, UserDeviceIdentifier};
use dashmap::DashMap;
use futures::{
  Stream,
  future::{self, FutureExt},
};
use getset::Getters;
use std::{
  collections::BTreeMap,
  convert::TryFrom,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};
use tokio::sync::{broadcast, mpsc};

#[derive(Debug)]
pub(super) enum DeviceManagerCommand {
  StartScanning,
  StopScanning,
}

#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct ServerDeviceInfo {
  identifier: UserDeviceIdentifier,
  display_name: Option<String>,
  needs_keepalive: bool,
}

pub struct ServerDeviceManagerBuilder {
  device_configuration_manager: Arc<DeviceConfigurationManager>,
  comm_managers: Vec<Box<dyn HardwareCommunicationManagerBuilder>>,
  emit_output_observations: bool,
}

impl ServerDeviceManagerBuilder {
  pub fn new(device_configuration_manager: DeviceConfigurationManager) -> Self {
    Self {
      device_configuration_manager: Arc::new(device_configuration_manager),
      comm_managers: vec![],
      emit_output_observations: false,
    }
  }

  /// Use a prebuilt device configuration manager that needs to be shared with the outside world
  /// (usually for serialization of user configurations to file)
  pub fn new_with_arc(device_configuration_manager: Arc<DeviceConfigurationManager>) -> Self {
    Self {
      device_configuration_manager,
      comm_managers: vec![],
      emit_output_observations: false,
    }
  }

  pub fn comm_manager<T>(&mut self, builder: T) -> &mut Self
  where
    T: HardwareCommunicationManagerBuilder + 'static,
  {
    self.comm_managers.push(Box::new(builder));
    self
  }

  pub fn emit_output_observations(&mut self, enabled: bool) -> &mut Self {
    self.emit_output_observations = enabled;
    self
  }

  pub fn add_simulated_devices_if_configured(&mut self) -> &mut Self {
    let simulated_devices = self.device_configuration_manager.simulated_devices();
    if !simulated_devices.is_empty() {
      use crate::device::hardware::simulated::{
        SimulatedDeviceEntry,
        SimulatedHardwareCommunicationManagerBuilder,
      };
      let entries: Vec<SimulatedDeviceEntry> = simulated_devices
        .iter()
        .map(|config| SimulatedDeviceEntry {
          identifier: config.identifier().clone(),
          display_name: config.display_name().clone(),
          address: config.address().clone(),
        })
        .collect();
      self.comm_manager(SimulatedHardwareCommunicationManagerBuilder::new(entries));
    }
    self
  }

  pub fn finish(&mut self) -> Result<ServerDeviceManager, ButtplugServerError> {
    let (device_command_sender, device_command_receiver) = mpsc::channel(256);
    let (device_event_sender, device_event_receiver) = mpsc::channel(256);
    let mut comm_managers: Vec<Box<dyn HardwareCommunicationManager>> = Vec::new();
    for builder in &mut self.comm_managers {
      let comm_mgr = builder.finish(device_event_sender.clone());

      if comm_managers
        .iter()
        .any(|mgr| mgr.name() == comm_mgr.name())
      {
        return Err(
          ButtplugServerError::DeviceCommunicationManagerTypeAlreadyAdded(
            comm_mgr.name().to_owned(),
          ),
        );
      }

      comm_managers.push(comm_mgr);
    }

    let mut colliding_dcms = vec![];
    for mgr in comm_managers.iter() {
      info!("{}: {}", mgr.name(), mgr.can_scan());
      // Hack: Lovense and Bluetooth dongles will fight with each other over devices, possibly
      // interrupting each other connecting and causing very weird issues for users. Print a
      // warning message to logs if more than one is active and available to scan.
      if [
        "BtlePlugCommunicationManager",
        "LovenseSerialDongleCommunicationManager",
        "LovenseHIDDongleCommunicationManager",
      ]
      .iter()
      .any(|x| x == &mgr.name())
        && mgr.can_scan()
      {
        colliding_dcms.push(mgr.name().to_owned());
      }
    }
    if colliding_dcms.len() > 1 {
      warn!(
        "The following device connection methods may collide: {}. This may mean you have lovense dongles and bluetooth dongles connected at the same time. Please disconnect the lovense dongles or turn off the Lovense HID/Serial Dongle support in Intiface/Buttplug. Lovense devices will work with the Bluetooth dongle.",
        colliding_dcms.join(", ")
      );
    }

    let devices = Arc::new(DashMap::new());
    let task_scope = TaskScope::root("device-manager");
    let devices_scope = task_scope.child("devices");

    let output_sender = broadcast::channel(255).0;
    let output_observation_sender = if self.emit_output_observations {
      Some(broadcast::channel(256).0)
    } else {
      None
    };

    // Clone everything the event loop needs, since the originals are still
    // required to construct the ServerDeviceManager below.
    let device_configuration_manager = self.device_configuration_manager.clone();
    let devices_clone = devices.clone();
    let output_sender_clone = output_sender.clone();
    let output_observation_sender_clone = output_observation_sender.clone();
    task_scope.spawn("event-loop", move |token| async move {
      let mut event_loop = ServerDeviceManagerEventLoop::new(
        comm_managers,
        device_configuration_manager,
        devices_clone,
        token,
        devices_scope,
        output_sender_clone,
        device_event_receiver,
        device_command_receiver,
        output_observation_sender_clone,
      );
      event_loop.run().await;
    });
    Ok(ServerDeviceManager {
      device_configuration_manager: self.device_configuration_manager.clone(),
      devices,
      device_command_sender,
      task_scope,
      running: Arc::new(AtomicBool::new(true)),
      output_sender,
      output_observation_sender,
    })
  }
}

#[derive(Getters)]
pub struct ServerDeviceManager {
  #[getset(get = "pub")]
  device_configuration_manager: Arc<DeviceConfigurationManager>,
  #[getset(get = "pub(crate)")]
  devices: Arc<DashMap<u32, DeviceHandle>>,
  device_command_sender: mpsc::Sender<DeviceManagerCommand>,
  task_scope: TaskScope,
  running: Arc<AtomicBool>,
  output_sender: broadcast::Sender<ButtplugServerMessageV4>,
  output_observation_sender: Option<broadcast::Sender<OutputObservation>>,
}

impl ServerDeviceManager {
  pub fn event_stream(&self) -> impl Stream<Item = ButtplugServerMessageV4> + use<> {
    // Unlike the client API, we can expect anyone using the server to pin this
    // themselves.
    convert_broadcast_receiver_to_stream(self.output_sender.subscribe())
  }

  pub fn output_observation_stream(&self) -> Option<impl Stream<Item = OutputObservation>> {
    self
      .output_observation_sender
      .as_ref()
      .map(|sender| convert_broadcast_receiver_to_stream(sender.subscribe()))
  }

  fn start_scanning(&self) -> ButtplugServerResultFuture {
    let command_sender = self.device_command_sender.clone();
    async move {
      if command_sender
        .send(DeviceManagerCommand::StartScanning)
        .await
        .is_err()
      {
        // TODO Fill in error.
      }
      Ok(message::OkV0::default().into())
    }
    .boxed()
  }

  fn stop_scanning(&self) -> ButtplugServerResultFuture {
    let command_sender = self.device_command_sender.clone();
    async move {
      if command_sender
        .send(DeviceManagerCommand::StopScanning)
        .await
        .is_err()
      {
        // TODO Fill in error.
      }
      Ok(message::OkV0::default().into())
    }
    .boxed()
  }

  pub(crate) fn stop_devices(&self, msg: &StopCmdV4) -> ButtplugServerResultFuture {
    let device_map = self.devices.clone();
    // TODO This could use some error reporting.
    let msg = msg.clone();
    async move {
      let fut_vec: Vec<_> = device_map
        .iter()
        .map(|dev| {
          let device = dev.value();
          device.stop(&message::StopCmdV4::new(
            None,
            None,
            msg.inputs(),
            msg.outputs(),
          ))
        })
        .collect();
      future::join_all(fut_vec).await;
      Ok(message::OkV0::default().into())
    }
    .boxed()
  }

  fn parse_device_message(
    &self,
    device_msg: ButtplugDeviceCommandMessageUnionV4,
  ) -> ButtplugServerResultFuture {
    match self.devices.get(&device_msg.device_index()) {
      Some(device) => {
        //let fut = device.parse_message(device_msg);
        device.parse_message(device_msg)
        // Create a future to run the message through the device, then handle adding the id to the result.
        //fut.boxed()
      }
      None => ButtplugDeviceError::DeviceNotAvailable(device_msg.device_index()).into(),
    }
  }

  fn generate_device_list(&self) -> DeviceListV4 {
    let devices = self
      .devices
      .iter()
      .map(|device| device.value().as_device_message_info(*device.key()))
      .collect();
    DeviceListV4::new(devices)
  }

  fn parse_device_manager_message(
    &self,
    manager_msg: ButtplugDeviceManagerMessageUnion,
  ) -> ButtplugServerResultFuture {
    match manager_msg {
      ButtplugDeviceManagerMessageUnion::RequestDeviceList(msg) => {
        let mut device_list = self.generate_device_list();
        device_list.set_id(msg.id());
        future::ready(Ok(device_list.into())).boxed()
      }
      ButtplugDeviceManagerMessageUnion::StopCmd(m) => self.stop_devices(&m),
      ButtplugDeviceManagerMessageUnion::StartScanning(_) => self.start_scanning(),
      ButtplugDeviceManagerMessageUnion::StopScanning(_) => self.stop_scanning(),
    }
  }

  pub fn parse_message(&self, msg: ButtplugCheckedClientMessageV4) -> ButtplugServerResultFuture {
    if !self.running.load(Ordering::Relaxed) {
      return future::ready(Err(ButtplugUnknownError::DeviceManagerNotRunning.into())).boxed();
    }
    // If this is a device command message, just route it directly to the
    // device.
    if let Ok(device_msg) = ButtplugDeviceCommandMessageUnionV4::try_from(msg.clone()) {
      self.parse_device_message(device_msg)
    } else if let Ok(manager_msg) = ButtplugDeviceManagerMessageUnion::try_from(msg.clone()) {
      self.parse_device_manager_message(manager_msg)
    } else {
      ButtplugMessageError::UnexpectedMessageType(format!("{msg:?}")).into()
    }
  }

  pub(crate) fn feature_map(&self) -> BTreeMap<u32, ServerDeviceAttributes> {
    self
      .devices()
      .iter()
      .map(|x| (*x.key(), x.legacy_attributes().clone()))
      .collect()
  }

  pub fn device_info(&self, index: u32) -> Option<ServerDeviceInfo> {
    self.devices.get(&index).map(|device| ServerDeviceInfo {
      identifier: device.value().identifier().clone(),
      display_name: device.value().definition().display_name().clone(),
      needs_keepalive: device.value().needs_keepalive(),
    })
  }

  // Only a ButtplugServer should be able to call this. We don't want to expose this capability to
  // the outside world. Note that this could cause issues for lifetimes if someone holds this longer
  // than the lifetime of the server that originally created it. Ideally we should lock the Server
  // Device Manager lifetime to the owning ButtplugServer lifetime to ensure that doesn't happen,
  // but that's going to be complicated.
  pub(crate) fn shutdown(&self) -> ButtplugServerResultFuture {
    let devices = self.devices.clone();
    // Make sure that, once our owning server shuts us down, no one outside can use this manager
    // again. Otherwise we can have all sorts of ownership weirdness.
    self.running.store(false, Ordering::Relaxed);
    let stop_scanning = self.stop_scanning();
    let stop_devices = self.stop_devices(&StopCmdV4::default());
    // TaskScope is not Clone, so capture its path and cancel here, then await
    // the subtree draining inside the returned future via the registry.
    let scope_path = self.task_scope.path().to_owned();
    self.task_scope.cancel();
    async move {
      // Force stop scanning, otherwise we can disconnect and instantly try to reconnect while
      // cleaning up if we're still scanning.
      let _ = stop_scanning.await;
      let _ = stop_devices.await;
      for device in devices.iter() {
        device.value().disconnect().await?;
      }
      buttplug_core::util::task::registry()
        .wait_empty_under(&scope_path)
        .await;
      Ok(message::OkV0::default().into())
    }
    .boxed()
  }
}

impl Drop for ServerDeviceManager {
  fn drop(&mut self) {
    info!("Dropping device manager!");
    // The task_scope field cancels its subtree on drop, so we only need to log
    // here; explicit cancellation happens automatically when the scope drops.
    self.task_scope.cancel();
  }
}
