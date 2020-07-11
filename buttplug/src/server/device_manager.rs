// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug Device Manager, manages Device Subtype (Platform/Communication bus
//! specific) Managers

use super::comm_managers::{
  DeviceCommunicationEvent, DeviceCommunicationManager, DeviceCommunicationManagerCreator,
};
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugMessageError, ButtplugUnknownError},
    messages::{
      self, ButtplugClientMessage, ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceManagerMessageUnion, ButtplugDeviceMessage, ButtplugMessage,
      ButtplugServerMessage, DeviceAdded, DeviceList, DeviceMessageInfo, DeviceRemoved,
      ScanningFinished,
    },
  },
  device::{ButtplugDevice, ButtplugDeviceEvent},
  server::ButtplugServerResultFuture,
  test::{TestDeviceCommunicationManager, TestDeviceCommunicationManagerHelper},
  util::async_manager,
};
use async_channel::{bounded, Receiver, Sender};
use async_mutex::Mutex;
use dashmap::DashMap;
use futures::{
  future::{self, Future},
  FutureExt, StreamExt,
};
use std::{
  convert::TryFrom,
  sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
  },
};

enum DeviceEvent {
  DeviceCommunicationEvent(Option<DeviceCommunicationEvent>),
  DeviceEvent(Option<(u32, ButtplugDeviceEvent)>),
  PingTimeout,
}

fn wait_for_manager_events(
  ping_receiver: Option<Receiver<()>>,
  server_sender: Sender<ButtplugServerMessage>,
) -> (
  impl Future<Output = ()>,
  Arc<DashMap<u32, ButtplugDevice>>,
  Sender<DeviceCommunicationEvent>,
) {
  let main_device_index = Arc::new(AtomicU32::new(0));
  let (device_event_sender, mut device_event_receiver) = bounded::<(u32, ButtplugDeviceEvent)>(256);
  let device_map = Arc::new(DashMap::new());
  let (device_comm_sender, mut device_comm_receiver) = bounded(256);
  let device_map_return = device_map.clone();
  let mut device_manager_status: Vec<Arc<AtomicBool>> = vec![];
  let event_loop = async move {
    loop {
      let ping_fut = async {
        if let Some(recv) = &ping_receiver {
          if recv.recv().await.is_err() {
            error!("Ping sender disappeared, meaning server has died. Exiting.");
          }
        } else {
          futures::future::pending::<()>().await;
        }
        // If the ping receiver ever gets anything, we've pinged out, so
        // just stop everything and exit.
        DeviceEvent::PingTimeout
      };

      let manager_event = select! {
        device_comm = device_comm_receiver.next().fuse() => DeviceEvent::DeviceCommunicationEvent(device_comm),
        device_event = device_event_receiver.next().fuse() => DeviceEvent::DeviceEvent(device_event),
        ping = ping_fut.fuse() => ping
      };

      match manager_event {
        DeviceEvent::DeviceCommunicationEvent(e) => match e {
          Some(event) => match event {
            DeviceCommunicationEvent::DeviceFound(device_creator) => {
              // Pull and increment the device index now. If connection fails,
              // we'll just iterate to the next one.
              let device_index = main_device_index.load(Ordering::SeqCst);
              main_device_index.store(
                main_device_index.load(Ordering::SeqCst) + 1,
                Ordering::SeqCst,
              );
              let device_event_sender_clone = device_event_sender.clone();
              let device_map_clone = device_map.clone();
              let server_sender_clone = server_sender.clone();
              async_manager::spawn(async move {
                match ButtplugDevice::try_create_device(device_creator).await {
                  Ok(option_dev) => match option_dev {
                    Some(device) => {
                      info!("Assigning index {} to {}", device_index, device.name());
                      let mut recv = device.get_event_receiver();
                      let sender_clone = device_event_sender_clone.clone();
                      let idx_clone = device_index;
                      async_manager::spawn(async move {
                        while let Some(e) = recv.next().await {
                          if sender_clone.send((idx_clone, e)).await.is_err() {
                            error!("Device event receiver disappeared, exiting loop.");
                            return;
                          }
                        }
                      })
                      .unwrap();
                      let device_added_message =
                        DeviceAdded::new(device_index, device.name(), &device.message_attributes());
                      device_map_clone.insert(device_index, device);
                      // After that, we can send out to the server's event
                      // listeners to let them know a device has been added.
                      if server_sender_clone
                        .send(device_added_message.into())
                        .await
                        .is_err()
                      {
                        error!("Server disappeared, exiting loop.");
                        return;
                      }
                    }
                    None => debug!("Device could not be matched to a protocol."),
                  },
                  Err(e) => error!("Device errored while trying to connect: {}", e),
                }
              })
              .unwrap();
            }
            DeviceCommunicationEvent::ScanningFinished => {
              for comm_mgr_status in &device_manager_status {
                if comm_mgr_status.load(Ordering::SeqCst) {
                  continue;
                }
              }
              if server_sender
                .send(ScanningFinished::default().into())
                .await
                .is_err()
              {
                error!("Server disappeared, exiting loop.");
                return;
              }
            }
            DeviceCommunicationEvent::DeviceManagerAdded(status) => {
              device_manager_status.push(status);
            }
          },
          None => break,
        },
        DeviceEvent::DeviceEvent(e) => match e {
          Some((idx, event)) => {
            if let ButtplugDeviceEvent::Removed = event {
              device_map.remove(&idx);
              if server_sender
                .send(DeviceRemoved::new(idx).into())
                .await
                .is_err()
              {
                error!("Server disappeared, exiting loop.");
                return;
              }
            }
            info!("Got device event: {:?}", event);
          }
          None => break,
        },
        DeviceEvent::PingTimeout => {
          error!("Pinged out, stopping devices");
          // read() is a write() lock here, so need to get through this ASAP. We
          // only write within this loop, but there's a chance that won't always
          // be the case.
          let fut_vec: Vec<_> = device_map
            .iter()
            .map(|dev| {
              let device = dev.value();
              device.parse_message(messages::StopDeviceCmd::new(1).into())
            })
            .collect();
          // TODO Should probably spawn this instead of blocking the loop.
          for fut in fut_vec {
            // Device index doesn't matter here, since we're sending the
            // message directly to the device itself.
            if let Err(e) = fut.await {
              error!("Error stopping device on ping timeout: {}", e);
            }
          }
          break;
        }
      }
    }
  };
  (event_loop, device_map_return, device_comm_sender)
}

pub struct DeviceManager {
  comm_managers: Arc<DashMap<String, Box<dyn DeviceCommunicationManager>>>,
  devices: Arc<DashMap<u32, ButtplugDevice>>,
  sender: Sender<DeviceCommunicationEvent>,
}

unsafe impl Send for DeviceManager {}

unsafe impl Sync for DeviceManager {}

impl DeviceManager {
  pub fn new(
    event_sender: Sender<ButtplugServerMessage>,
    ping_receiver: Option<Receiver<()>>,
  ) -> Self {
    let (event_loop_fut, device_map, device_event_sender) =
      wait_for_manager_events(ping_receiver, event_sender);
    async_manager::spawn(event_loop_fut).unwrap();
    Self {
      sender: device_event_sender,
      devices: device_map,
      comm_managers: Arc::new(DashMap::new()),
    }
  }

  fn start_scanning(&self) -> ButtplugServerResultFuture {
    if self.comm_managers.is_empty() {
      ButtplugUnknownError::NoDeviceCommManagers.into()
    } else {
      let mgrs = self.comm_managers.clone();
      Box::pin(async move {
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
    match self.devices.get(&device_msg.get_device_index()) {
      Some(device) => {
        let fut = device.parse_message(device_msg);
        // Create a future to run the message through the device, then handle adding the id to the result.
        Box::pin(async move { fut.await })
      }
      None => ButtplugDeviceError::DeviceNotAvailable(device_msg.get_device_index()).into(),
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
            DeviceMessageInfo {
              device_index: *device.key(),
              device_name: dev.name().to_string(),
              device_messages: dev.message_attributes(),
            }
          })
          .collect();
        let mut device_list = DeviceList::new(devices);
        device_list.set_id(msg.get_id());
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

  pub fn add_comm_manager<T>(&mut self)
  where
    T: 'static + DeviceCommunicationManager + DeviceCommunicationManagerCreator,
  {
    let mgr = T::new(self.sender.clone());
    let status = mgr.scanning_status();
    let sender = self.sender.clone();
    // TODO This could run out of order and possibly cause weird scanning finished bugs?
    async_manager::spawn(async move {
      sender
        .send(DeviceCommunicationEvent::DeviceManagerAdded(status))
        .await
        .unwrap();
    }).unwrap();
    self.comm_managers.insert(mgr.name().to_owned(), Box::new(mgr));
  }

  pub fn add_test_comm_manager(&mut self) -> TestDeviceCommunicationManagerHelper {
    let mgr = TestDeviceCommunicationManager::new(self.sender.clone());
    let status = mgr.scanning_status();
    let sender = self.sender.clone();
    // TODO This could run out of order and possibly cause weird scanning finished bugs?
    async_manager::spawn(async move {
      sender
        .send(DeviceCommunicationEvent::DeviceManagerAdded(status))
        .await
        .unwrap();
    }).unwrap();
    let helper = mgr.helper();
    self.comm_managers.insert(mgr.name().to_owned(), Box::new(mgr));
    helper
  }
}

impl Drop for DeviceManager {
  fn drop(&mut self) {
    info!("Dropping device manager!");
  }
}
