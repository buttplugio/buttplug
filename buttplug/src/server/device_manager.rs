// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug Device Manager, manages Device Subtype (Platform/Communication bus
//! specific) Managers

use super::{
  comm_managers::{
    DeviceCommunicationEvent,
    DeviceCommunicationManager,
    DeviceCommunicationManagerBuilder,
  },
  device_manager_event_loop::DeviceManagerEventLoop,
  ping_timer::PingTimer,
  ButtplugServerError,
};
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
  device::{
    configuration_manager::{DeviceConfigurationManager, ProtocolDeviceConfiguration},
    protocol::ButtplugProtocolFactory,
    ButtplugDevice,
  },
  server::ButtplugServerResultFuture,
  util::async_manager,
};
use dashmap::{DashMap, DashSet};
use futures::future;
use std::{
  convert::TryFrom,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use tokio::sync::{broadcast, mpsc};

#[derive(Debug)]
pub struct DeviceInfo {
  pub address: String,
  pub display_name: Option<String>,
}

pub struct DeviceManager {
  // This uses a map to make sure we don't have 2 comm managers of the same type
  // register. Also means we can do lockless access since it's a Dashmap.
  comm_managers: Arc<DashMap<String, Box<dyn DeviceCommunicationManager>>>,
  devices: Arc<DashMap<u32, Arc<ButtplugDevice>>>,
  device_event_sender: mpsc::Sender<DeviceCommunicationEvent>,
  config: Arc<DeviceConfigurationManager>,
  has_run_first_scan_status: Arc<AtomicBool>,
  /// List of device addresses we are ONLY allowed to connect to. If no addresses available, connect
  /// to any device.
  allowed_devices: Arc<DashSet<String>>,
  /// List of device addresses we are not allowed to connect to.
  denied_devices: Arc<DashSet<String>>,
  /// Map of device address to related index, if index is stored.
  reserved_device_indexes: Arc<DashMap<String, u32>>
}

impl DeviceManager {
  pub fn new(
    output_sender: broadcast::Sender<ButtplugServerMessage>,
    ping_timer: Arc<PingTimer>,
    allow_raw_messages: bool,
  ) -> Self {
    let config = Arc::new(DeviceConfigurationManager::new(allow_raw_messages));
    let devices = Arc::new(DashMap::new());
    let (device_event_sender, device_event_receiver) = mpsc::channel(256);
    let allowed_devices = Arc::new(DashSet::new());
    let denied_devices = Arc::new(DashSet::new());
    let reserved_device_indexes = Arc::new(DashMap::new());
    let mut event_loop = DeviceManagerEventLoop::new(
      config.clone(),
      output_sender,
      devices.clone(),
      ping_timer,
      device_event_receiver,
      allowed_devices.clone(),
      denied_devices.clone(),
      reserved_device_indexes.clone()
    );
    async_manager::spawn(async move {
      event_loop.run().await;
    });
    Self {
      device_event_sender,
      devices,
      comm_managers: Arc::new(DashMap::new()),
      config,
      has_run_first_scan_status: Arc::new(AtomicBool::new(false)),
      allowed_devices,
      denied_devices,
      reserved_device_indexes
    }
  }

  fn start_scanning(&self) -> ButtplugServerResultFuture {
    if self.comm_managers.is_empty() {
      ButtplugUnknownError::NoDeviceCommManagers.into()
    } else {
      let mgrs = self.comm_managers.clone();
      let sender = self.device_event_sender.clone();
      let has_run_first_scan_status = self.has_run_first_scan_status.clone();
      Box::pin(async move {
        if !has_run_first_scan_status.load(Ordering::SeqCst) {
          info!("Scanning Status (Only shown on first scan)");
          let mut colliding_dcms = vec![];
          for mgr in mgrs.iter() {
            info!("{}: {}", mgr.key(), mgr.value().can_scan());
            // Hack: Lovense and Bluetooth dongles will fight with each other over devices, possibly
            // interrupting each other connecting and causing very weird issues for users. Print a
            // warning message to logs if more than one is active and available to scan.
            if (mgr.key() == "BtlePlugCommunicationManager"
              || mgr.key() == "LovenseSerialDongleCommunicationManager"
              || mgr.key() == "LovenseHIDDongleCommunicationManager")
              && mgr.value().can_scan()
            {
              colliding_dcms.push(mgr.key().clone());
            }
          }
          if colliding_dcms.len() > 1 {
            warn!("The following device connection methods may collide: {}. This may mean you have lovense dongles and bluetooth dongles connected at the same time. Please disconnect the lovense dongles or turn off the Lovense HID/Serial Dongle support in Intiface/Buttplug. Lovense devices will work with the Bluetooth dongle.", colliding_dcms.join(", "));
          }
          has_run_first_scan_status.store(true, Ordering::SeqCst);
        }

        for mgr in mgrs.iter() {
          if mgr.value().scanning_status().load(Ordering::SeqCst) {
            return Err(ButtplugDeviceError::DeviceScanningAlreadyStarted.into());
          }
        }
        let fut_vec: Vec<_> = mgrs
          .iter()
          .map(|guard| guard.value().start_scanning())
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
          if mgr.value().scanning_status().load(Ordering::SeqCst) {
            debug!("Device manager {} has not stopped scanning yet.", mgr.key());
            scanning_stopped = false;
            break;
          }
        }
        if scanning_stopped {
          return Err(ButtplugDeviceError::DeviceScanningAlreadyStopped.into());
        }

        let fut_vec: Vec<_> = mgrs
          .iter()
          .map(|guard| guard.value().stop_scanning())
          .collect();
        // TODO If stop_scanning fails anywhere, this will ignore it. We should maybe at least log?
        future::join_all(fut_vec).await;
        Ok(messages::Ok::default().into())
      })
    }
  }

  fn stop_all_devices(&self) -> ButtplugServerResultFuture {
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

  pub fn add_comm_manager<T>(&self, builder: T) -> Result<(), ButtplugServerError>
  where
    T: DeviceCommunicationManagerBuilder,
  {
    let mgr = builder
      .event_sender(self.device_event_sender.clone())
      .finish();
    if self.comm_managers.contains_key(mgr.name()) {
      return Err(ButtplugServerError::DeviceManagerTypeAlreadyAdded(
        mgr.name().to_owned(),
      ));
    }
    let status = mgr.scanning_status();
    let sender = self.device_event_sender.clone();
    // TODO This could run out of order and possibly cause weird scanning finished bugs?
    async_manager::spawn(async move {
      sender
        .send(DeviceCommunicationEvent::DeviceManagerAdded(status))
        .await
        .expect("We should always have an event loop for this to go to.");
    });
    info!("Added device comm manager {}", mgr.name().to_owned());
    self.comm_managers.insert(mgr.name().to_owned(), mgr);
    Ok(())
  }

  pub fn add_allowed_device(&self, address: &str) {
    self.allowed_devices.insert(address.to_owned());
  }

  pub fn remove_allowed_device(&self, address: &str) {
    self.allowed_devices.remove(address);
  }

  pub fn add_denied_device(&self, address: &str) {
    self.denied_devices.insert(address.to_owned());
  }

  pub fn remove_denied_device(&self, address: &str) {
    self.denied_devices.remove(address);
  }

  pub fn add_reserved_device_index(&self, address: &str, index: u32) {
    self.reserved_device_indexes.insert(address.to_owned(), index);
  }

  pub fn remove_reserved_device_index(&self, address: &str) {
    self.reserved_device_indexes.remove(address);
  }

  pub fn add_protocol_factory<T>(&self, factory: T) -> Result<(), ButtplugServerError>
  where
    T: ButtplugProtocolFactory + 'static,
  {
    let protocol_identifier = factory.protocol_identifier().to_owned();
    self.config.add_protocol_factory(factory).map_err(|_| ButtplugServerError::ProtocolAlreadyAdded(protocol_identifier))
  }

  pub fn remove_protocol_factory<T>(&self, protocol_identifier: &str) -> Result<(), ButtplugServerError> 
  where
    T: ButtplugProtocolFactory + 'static 
  {
    self.config.remove_protocol_factory(protocol_identifier).map_err(|_| ButtplugServerError::ProtocolDoesNotExist(protocol_identifier.to_owned()))
  }

  pub fn remove_all_protocol_factories(&self) {
    info!("Removed all protocols");
    self.config.remove_all_protocol_factories();
  }

  pub fn add_protocol_device_configuration(&self, name: &str, config: &ProtocolDeviceConfiguration) -> Result<(), ButtplugError> {
    info!("Adding protocol device config {}", name);
    self.config.add_protocol_device_configuration(name, config)
  }

  pub fn remove_protocol_device_configuration(&self, name: &str) {
    info!("Removing protocol device config {}", name);
    self.config.remove_protocol_device_configuration(name);
  }

  pub fn device_info(&self, index: u32) -> Result<DeviceInfo, ButtplugDeviceError> {
    if let Some(device) = self.devices.get(&index) {
      Ok(DeviceInfo {
        address: device.value().device_identifier().to_owned(),
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
  }
}
