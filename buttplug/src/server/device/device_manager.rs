// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug Device Manager, manages Device Subtype (Platform/Communication bus
//! specific) Managers

use super::device_manager_event_loop::DeviceManagerEventLoop;
use crate::{
  core::{
    errors::{ButtplugError, ButtplugDeviceError, ButtplugMessageError, ButtplugUnknownError},
    messages::{
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
  server::device::{
    configuration::{DeviceConfigurationManagerBuilder, ProtocolDeviceConfiguration, ProtocolDeviceIdentifier},
    protocol::ButtplugProtocolFactory,
    hardware::{
      communication::{
        DeviceCommunicationEvent,
        DeviceCommunicationManager,
        DeviceCommunicationManagerBuilder,
      },
    },
    ServerDevice,
  },
  server::{
    ButtplugServerResultFuture,
  },  
  util::async_manager,
};
use dashmap::DashMap;
use futures::future;
use tokio_util::sync::CancellationToken;
use std::{
  convert::TryFrom,
  sync::{
    atomic::Ordering,
    Arc,
  },
};
use tokio::sync::{broadcast, mpsc};

#[derive(Debug)]
pub struct DeviceInfo {
  pub identifier: ProtocolDeviceIdentifier,
  pub display_name: Option<String>,
}

#[derive(Default)]
pub struct DeviceManagerBuilder {
  configuration_manager_builder: DeviceConfigurationManagerBuilder,
  comm_managers: Vec<Box<dyn DeviceCommunicationManagerBuilder>>,
}

impl DeviceManagerBuilder {
  pub fn comm_manager<T>(&mut self, builder: T) -> &mut Self
  where
    T: DeviceCommunicationManagerBuilder + 'static,
  {    
    self.comm_managers.push(Box::new(builder));
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

  pub fn reserved_index(&mut self, identifier: &ProtocolDeviceIdentifier, index: u32) -> &mut Self {
    self.configuration_manager_builder.reserved_index(identifier, index);
    self
  }

  pub fn protocol_factory<T>(&mut self, factory: T) -> &mut Self
  where
    T: ButtplugProtocolFactory + 'static,
  {
    self.configuration_manager_builder.protocol_factory(factory);
    self
  }

  pub fn protocol_device_configuration(&mut self, name: &str, config: &ProtocolDeviceConfiguration) -> &mut Self {
    self.configuration_manager_builder.protocol_device_configuration(name, config);
    self
  }

  pub fn no_default_protocols(&mut self) -> &mut Self {
    self.configuration_manager_builder.no_default_protocols();
    self
  }
  
  pub fn allow_raw_messages(&mut self) -> &mut Self {
    
    self.configuration_manager_builder.allow_raw_messages();
    self
  }

  pub fn finish(&mut self, output_sender: broadcast::Sender<ButtplugServerMessage>) -> Result<DeviceManager, ButtplugError> {
    let config_mgr = self.configuration_manager_builder.finish()?;

    let (device_event_sender, device_event_receiver) = mpsc::channel(256);
    let mut comm_managers = Vec::new();
    for builder in &self.comm_managers {
      let comm_mgr = builder.finish(device_event_sender.clone());

      if comm_managers.iter().any(|mgr: &Box<dyn DeviceCommunicationManager>| &mgr.name() == &comm_mgr.name()) {
        // TODO Fill in error
      }

      comm_managers.push(comm_mgr);
    }
    
    let mut colliding_dcms = vec![];
    for mgr in comm_managers.iter() {
      info!("{}: {}", mgr.name(), mgr.can_scan());
      // Hack: Lovense and Bluetooth dongles will fight with each other over devices, possibly
      // interrupting each other connecting and causing very weird issues for users. Print a
      // warning message to logs if more than one is active and available to scan.
      if ["BtlePlugCommunicationManager", "LovenseSerialDongleCommunicationManager", "LovenseHIDDongleCommunicationManager"].iter().any(|x| x == &mgr.name())
        && mgr.can_scan()
      {
        colliding_dcms.push(mgr.name().clone());
      }
    }
    if colliding_dcms.len() > 1 {
      warn!("The following device connection methods may collide: {}. This may mean you have lovense dongles and bluetooth dongles connected at the same time. Please disconnect the lovense dongles or turn off the Lovense HID/Serial Dongle support in Intiface/Buttplug. Lovense devices will work with the Bluetooth dongle.", colliding_dcms.join(", "));
    }

    let devices = Arc::new(DashMap::new());
    let loop_cancellation_token = CancellationToken::new();

    let mut event_loop = DeviceManagerEventLoop::new(
      config_mgr,
      devices.clone(),
      loop_cancellation_token.child_token(),
      output_sender,
      device_event_receiver,
    );
    async_manager::spawn(async move {
      event_loop.run().await;
    });
    Ok(DeviceManager {
      comm_managers: Arc::new(comm_managers),
      devices,
      device_event_sender,
      loop_cancellation_token
    })
  }
}

pub struct DeviceManager {
  // This uses a map to make sure we don't have 2 comm managers of the same type
  // register. Also means we can do lockless access since it's a Dashmap.
  comm_managers: Arc<Vec<Box<dyn DeviceCommunicationManager>>>,
  devices: Arc<DashMap<u32, Arc<ServerDevice>>>,
  device_event_sender: mpsc::Sender<DeviceCommunicationEvent>,
  loop_cancellation_token: CancellationToken
}

impl DeviceManager {
  fn start_scanning(&self) -> ButtplugServerResultFuture {
    if self.comm_managers.is_empty() {
      ButtplugUnknownError::NoDeviceCommManagers.into()
    } else {
      let mgrs = self.comm_managers.clone();
      let sender = self.device_event_sender.clone();
      Box::pin(async move {
        // TODO Does this really matter? If we're already scanning, who cares?
        for mgr in mgrs.iter() {
          if mgr.scanning_status().load(Ordering::SeqCst) {
            warn!("Scanning still in progress, returning.");
            return Ok(messages::Ok::default().into());
          }
        }
        info!("No scan currently in progress, starting new scan.");
        let fut_vec: Vec<_> = mgrs
          .iter()
          .map(|guard| guard.start_scanning())
          .collect();
        // TODO If start_scanning fails anywhere, this will ignore it. We should maybe at least log?
        future::join_all(fut_vec).await;
        debug!("All managers started, sending ScanningStarted (and invoking ScanningFinished hack) signal to event loop.");
        // HACK: In case everything somehow exited between the time all of our
        // futures resolved and when we updated the event loop, act like we're a
        // device comm manager and send a ScanningFinished message. This will
        // cause the finish check to run just in case, so we don't get stuck.
        //
        // Ideally, this should be some sort of state machine, but for now, we
        // can deal with this.
        //
        // At this point, it doesn't really matter what we return, only way that
        // event loop could shut down is if the whole system is shutting down.
        // So complain if our sends error out, but don't worry about returning
        // an error.
        if sender
          .send(DeviceCommunicationEvent::ScanningStarted)
          .await
          .is_err()
          || sender
            .send(DeviceCommunicationEvent::ScanningFinished)
            .await
            .is_err()
        {
          debug!("Device manager event loop shut down, cannot send ScanningStarted");
        }
        Ok(messages::Ok::default().into())
      })
    }
  }

  fn stop_scanning(&self) -> ButtplugServerResultFuture {
    if self.comm_managers.is_empty() {
      ButtplugUnknownError::NoDeviceCommManagers.into()
    } else {
      let mgrs = self.comm_managers.clone();
      Box::pin(async move {
        let mut scanning_stopped = true;
        for mgr in mgrs.iter() {
          if mgr.scanning_status().load(Ordering::SeqCst) {
            debug!("Device manager {} has not stopped scanning yet.", mgr.name());
            scanning_stopped = false;
            break;
          }
        }
        if scanning_stopped {
          return Err(ButtplugDeviceError::DeviceScanningAlreadyStopped.into());
        }

        let fut_vec: Vec<_> = mgrs
          .iter()
          .map(|guard| guard.stop_scanning())
          .collect();
        // TODO If stop_scanning fails anywhere, this will ignore it. We should maybe at least log?
        future::join_all(fut_vec).await;
        Ok(messages::Ok::default().into())
      })
    }
  }

  pub(crate) fn stop_all_devices(&self) -> ButtplugServerResultFuture {
    let device_map = self.devices.clone();
    // TODO This could use some error reporting.
    Box::pin(async move {
      let fut_vec: Vec<_> = device_map
        .iter()
        .map(|dev| {
          let device = dev.value();
          device.parse_message(messages::StopDeviceCmd::new(1).into())
        })
        .collect();
      future::join_all(fut_vec).await;
      Ok(messages::Ok::default().into())
    })
  }

  fn parse_device_message(
    &self,
    device_msg: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugServerResultFuture {
    match self.devices.get(&device_msg.device_index()) {
      Some(device) => {
        let fut = device.parse_message(device_msg);
        // Create a future to run the message through the device, then handle adding the id to the result.
        Box::pin(async move { fut.await })
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
            DeviceMessageInfo::new(*device.key(), &dev.name(), dev.message_attributes())
          })
          .collect();
        let mut device_list = DeviceList::new(devices);
        device_list.set_id(msg.id());
        Box::pin(future::ready(Ok(device_list.into())))
      }
      ButtplugDeviceManagerMessageUnion::StopAllDevices(_) => self.stop_all_devices(),
      ButtplugDeviceManagerMessageUnion::StartScanning(_) => self.start_scanning(),
      ButtplugDeviceManagerMessageUnion::StopScanning(_) => self.stop_scanning(),
    }
  }

  pub fn parse_message(&self, msg: ButtplugClientMessage) -> ButtplugServerResultFuture {
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

  pub fn device_info(&self, index: u32) -> Result<DeviceInfo, ButtplugDeviceError> {
    if let Some(device) = self.devices.get(&index) {
      Ok(DeviceInfo {
        identifier: device.value().device_identifier().clone(),
        display_name: device.value().display_name(),
      })
    } else {
      Err(ButtplugDeviceError::DeviceNotAvailable(index))
    }
  }
}

impl Drop for DeviceManager {
  fn drop(&mut self) {
    info!("Dropping device manager!");
    self.loop_cancellation_token.cancel();
  }
}
