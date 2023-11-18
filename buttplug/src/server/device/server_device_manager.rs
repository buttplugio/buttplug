// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug Device Manager, manages Device Subtype (Platform/Communication bus
//! specific) Managers

use super::server_device_manager_event_loop::ServerDeviceManagerEventLoop;
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugMessageError, ButtplugUnknownError},
    message::{
      self,
      ButtplugClientMessage,
      ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceManagerMessageUnion,
      ButtplugDeviceMessage,
      ButtplugMessage,
      ButtplugServerMessage,
      DeviceList,
      DeviceMessageInfo,
    },
  },
  server::{
    device::{
      configuration::{
        DeviceConfigurationManagerBuilder,
        ProtocolAttributesIdentifier,
        ProtocolCommunicationSpecifier,
        ProtocolDeviceAttributes,
      },
      hardware::communication::{
        HardwareCommunicationManager,
        HardwareCommunicationManagerBuilder,
      },
      protocol::ProtocolIdentifierFactory,
      ServerDevice,
      ServerDeviceIdentifier,
    },
    ButtplugServerError,
    ButtplugServerResultFuture,
  },
  util::{async_manager, stream::convert_broadcast_receiver_to_stream},
};
use dashmap::DashMap;
use futures::{
  future::{self, FutureExt},
  Stream,
};
use getset::Getters;
use std::{
  convert::TryFrom,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub(super) enum DeviceManagerCommand {
  StartScanning,
  StopScanning,
}

#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct ServerDeviceInfo {
  identifier: ServerDeviceIdentifier,
  display_name: Option<String>,
}

#[derive(Default)]
pub struct ServerDeviceManagerBuilder {
  configuration_manager_builder: DeviceConfigurationManagerBuilder,
  comm_managers: Vec<Box<dyn HardwareCommunicationManagerBuilder>>,
}

impl ServerDeviceManagerBuilder {
  pub fn comm_manager<T>(&mut self, builder: T) -> &mut Self
  where
    T: HardwareCommunicationManagerBuilder + 'static,
  {
    self.comm_managers.push(Box::new(builder));
    self
  }

  pub fn device_configuration_manager_builder(
    &mut self,
    dcm_builder: &DeviceConfigurationManagerBuilder,
  ) -> &mut Self {
    self.configuration_manager_builder.merge(dcm_builder);
    self
  }

  pub fn allowed_address(&mut self, address: &str) -> &mut Self {
    self.configuration_manager_builder.allowed_address(address);
    self
  }

  pub fn denied_address(&mut self, address: &str) -> &mut Self {
    self.configuration_manager_builder.denied_address(address);
    self
  }

  pub fn reserved_index(&mut self, identifier: &ServerDeviceIdentifier, index: u32) -> &mut Self {
    self
      .configuration_manager_builder
      .reserved_index(identifier, index);
    self
  }

  pub fn protocol_factory<T>(&mut self, factory: T) -> &mut Self
  where
    T: ProtocolIdentifierFactory + 'static,
  {
    self.configuration_manager_builder.protocol_factory(factory);
    self
  }

  pub fn communication_specifier(
    &mut self,
    protocol_name: &str,
    specifier: ProtocolCommunicationSpecifier,
  ) -> &mut Self {
    self
      .configuration_manager_builder
      .communication_specifier(protocol_name, specifier);
    self
  }

  pub fn protocol_attributes(
    &mut self,
    identifier: ProtocolAttributesIdentifier,
    attributes: ProtocolDeviceAttributes,
  ) -> &mut Self {
    self
      .configuration_manager_builder
      .protocol_attributes(identifier, attributes);
    self
  }

  pub fn skip_default_protocols(&mut self) -> &mut Self {
    self.configuration_manager_builder.skip_default_protocols();
    self
  }

  pub fn allow_raw_messages(&mut self) -> &mut Self {
    self.configuration_manager_builder.allow_raw_messages();
    self
  }

  pub fn finish(&mut self) -> Result<ServerDeviceManager, ButtplugServerError> {
    let config_mgr = self
      .configuration_manager_builder
      .finish()
      .map_err(ButtplugServerError::DeviceConfigurationManagerError)?;

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
      warn!("The following device connection methods may collide: {}. This may mean you have lovense dongles and bluetooth dongles connected at the same time. Please disconnect the lovense dongles or turn off the Lovense HID/Serial Dongle support in Intiface/Buttplug. Lovense devices will work with the Bluetooth dongle.", colliding_dcms.join(", "));
    }

    let devices = Arc::new(DashMap::new());
    let loop_cancellation_token = CancellationToken::new();

    let output_sender = broadcast::channel(255).0;

    let mut event_loop = ServerDeviceManagerEventLoop::new(
      comm_managers,
      config_mgr,
      devices.clone(),
      loop_cancellation_token.child_token(),
      output_sender.clone(),
      device_event_receiver,
      device_command_receiver,
    );
    async_manager::spawn(async move {
      event_loop.run().await;
    });
    Ok(ServerDeviceManager {
      devices,
      device_command_sender,
      loop_cancellation_token,
      running: Arc::new(AtomicBool::new(true)),
      output_sender,
    })
  }
}

pub struct ServerDeviceManager {
  devices: Arc<DashMap<u32, Arc<ServerDevice>>>,
  device_command_sender: mpsc::Sender<DeviceManagerCommand>,
  loop_cancellation_token: CancellationToken,
  running: Arc<AtomicBool>,
  output_sender: broadcast::Sender<ButtplugServerMessage>,
}

impl ServerDeviceManager {
  pub fn event_stream(&self) -> impl Stream<Item = ButtplugServerMessage> {
    // Unlike the client API, we can expect anyone using the server to pin this
    // themselves.
    convert_broadcast_receiver_to_stream(self.output_sender.subscribe())
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
      Ok(message::Ok::default().into())
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
      Ok(message::Ok::default().into())
    }
    .boxed()
  }

  pub(crate) fn stop_all_devices(&self) -> ButtplugServerResultFuture {
    let device_map = self.devices.clone();
    // TODO This could use some error reporting.
    async move {
      let fut_vec: Vec<_> = device_map
        .iter()
        .map(|dev| {
          let device = dev.value();
          device.parse_message(message::StopDeviceCmd::new(1).into())
        })
        .collect();
      future::join_all(fut_vec).await;
      Ok(message::Ok::default().into())
    }
    .boxed()
  }

  fn parse_device_message(
    &self,
    device_msg: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugServerResultFuture {
    match self.devices.get(&device_msg.device_index()) {
      Some(device) => {
        let fut = device.parse_message(device_msg);
        // Create a future to run the message through the device, then handle adding the id to the result.
        async move { fut.await }.boxed()
      }
      None => ButtplugDeviceError::DeviceNotAvailable(device_msg.device_index()).into(),
    }
  }

  fn parse_device_manager_message(
    &self,
    manager_msg: ButtplugDeviceManagerMessageUnion,
  ) -> ButtplugServerResultFuture {
    match manager_msg {
      ButtplugDeviceManagerMessageUnion::RequestDeviceList(msg) => {
        let devices = self
          .devices
          .iter()
          .map(|device| {
            let dev = device.value();
            DeviceMessageInfo::new(
              *device.key(),
              &dev.name(),
              &dev.display_name(),
              &None,
              dev.message_attributes().into(),
            )
          })
          .collect();
        let mut device_list = DeviceList::new(devices);
        device_list.set_id(msg.id());
        future::ready(Ok(device_list.into())).boxed()
      }
      ButtplugDeviceManagerMessageUnion::StopAllDevices(_) => self.stop_all_devices(),
      ButtplugDeviceManagerMessageUnion::StartScanning(_) => self.start_scanning(),
      ButtplugDeviceManagerMessageUnion::StopScanning(_) => self.stop_scanning(),
    }
  }

  pub fn parse_message(&self, msg: ButtplugClientMessage) -> ButtplugServerResultFuture {
    if !self.running.load(Ordering::SeqCst) {
      return future::ready(Err(ButtplugUnknownError::DeviceManagerNotRunning.into())).boxed();
    }
    // If this is a device command message, just route it directly to the
    // device.
    match ButtplugDeviceCommandMessageUnion::try_from(msg.clone()) {
      Ok(device_msg) => self.parse_device_message(device_msg),
      Err(_) => match ButtplugDeviceManagerMessageUnion::try_from(msg.clone()) {
        Ok(manager_msg) => self.parse_device_manager_message(manager_msg),
        Err(_) => ButtplugMessageError::UnexpectedMessageType(format!("{:?}", msg)).into(),
      },
    }
  }

  pub fn device_info(&self, index: u32) -> Option<ServerDeviceInfo> {
    self.devices.get(&index).map(|device| ServerDeviceInfo {
      identifier: device.value().identifier().clone(),
      display_name: device.value().display_name(),
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
    self.running.store(false, Ordering::SeqCst);
    let stop_scanning = self.stop_scanning();
    let stop_devices = self.stop_all_devices();
    let token = self.loop_cancellation_token.clone();
    async move {
      // Force stop scanning, otherwise we can disconnect and instantly try to reconnect while
      // cleaning up if we're still scanning.
      let _ = stop_scanning.await;
      let _ = stop_devices.await;
      for device in devices.iter() {
        device.value().disconnect().await?;
      }
      token.cancel();
      Ok(message::Ok::default().into())
    }
    .boxed()
  }
}

impl Drop for ServerDeviceManager {
  fn drop(&mut self) {
    info!("Dropping device manager!");
    self.loop_cancellation_token.cancel();
  }
}
