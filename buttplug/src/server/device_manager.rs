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
    DeviceCommunicationManagerCreator,
  },
  ButtplugServerStartupError,
};
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugMessageError, ButtplugUnknownError},
    messages::{
      self,
      ButtplugClientMessage,
      ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceManagerMessageUnion,
      ButtplugDeviceMessage,
      ButtplugMessage,
      ButtplugServerMessage,
      DeviceAdded,
      DeviceList,
      DeviceMessageInfo,
      DeviceRemoved,
      ScanningFinished,
    },
  },
  device::{
    configuration_manager::DeviceConfigurationManager,
    ButtplugDevice,
    ButtplugDeviceEvent,
  },
  server::ButtplugServerResultFuture,
  test::{TestDeviceCommunicationManager, TestDeviceCommunicationManagerHelper},
  util::async_manager,
};
use async_channel::{bounded, Receiver, Sender};
use async_lock::Semaphore;
use dashmap::DashMap;
use futures::{
  future::{self, Future},
  FutureExt,
  StreamExt,
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
  device_config_manager: Arc<DeviceConfigurationManager>,
  ping_receiver: Option<Receiver<()>>,
  server_sender: Sender<ButtplugServerMessage>,
) -> (
  impl Future<Output = ()>,
  Arc<DashMap<u32, ButtplugDevice>>,
  Sender<DeviceCommunicationEvent>,
) {
  let main_device_index = Arc::new(AtomicU32::new(0));
  let device_index_map: Arc<DashMap<String, u32>> = Arc::new(DashMap::new());
  let (device_event_sender, mut device_event_receiver) = bounded::<(u32, ButtplugDeviceEvent)>(256);
  let device_map = Arc::new(DashMap::new());
  let (device_comm_sender, mut device_comm_receiver) = bounded(256);
  let device_map_return = device_map.clone();
  let mut device_manager_status: Vec<Arc<AtomicBool>> = vec![];
  let device_addition_semaphore = Arc::new(Semaphore::new(1));
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
              let generated_device_index = main_device_index.load(Ordering::SeqCst);
              main_device_index.store(
                main_device_index.load(Ordering::SeqCst) + 1,
                Ordering::SeqCst,
              );
              debug!("Current generated device index: {}", generated_device_index);
              let device_event_sender_clone = device_event_sender.clone();
              let device_map_clone = device_map.clone();
              let server_sender_clone = server_sender.clone();
              let device_config_mgr_clone = device_config_manager.clone();
              let device_index_map_clone = device_index_map.clone();
              let device_addition_semaphore_clone = device_addition_semaphore.clone();
              async_manager::spawn(async move {
                match ButtplugDevice::try_create_device(device_config_mgr_clone, device_creator)
                  .await
                {
                  Ok(option_dev) => match option_dev {
                    Some(device) => {
                      // In order to make sure we don't collide IDs, we can only
                      // insert one device at a time. So much for lockless
                      // buttplugs. :(
                      let _guard = device_addition_semaphore_clone.acquire_arc().await;
                      // See if we have a reusable device index here.
                      let device_index =
                        if let Some(id) = device_index_map_clone.get(device.address()) {
                          id.value().clone()
                        } else {
                          device_index_map_clone
                            .insert(device.address().to_owned(), generated_device_index);
                          generated_device_index
                        };
                      // Since we can now reuse device indexes, this means we
                      // might possibly stomp on devices already in the map if
                      // they don't register a disconnect before we try to
                      // insert the new device. If we have a device already in
                      // the map with the same index (and therefore same
                      // address), consider it disconnected and eject it from
                      // the map. This should also trigger a disconnect event
                      // before our new DeviceAdded message goes out, so timing
                      // matters here.
                      if device_map_clone.contains_key(&device_index) {
                        info!("Device map contains key!");
                        // We just checked that the key exists, so we can unwrap
                        // here.
                        let (_, old_device): (_, ButtplugDevice) =
                          device_map_clone.remove(&device_index).unwrap();
                        // After removing the device from the array, manually
                        // disconnect it to make sure the event is thrown.
                        if let Err(err) = old_device.disconnect().await {
                          // If we throw an error during the disconnect, we
                          // can't really do anything with it, but should at
                          // least log it.
                          error!("Error during index collision disconnect: {:?}", err);
                        }
                      } else {
                        info!("Device map does not contain key!");
                      }

                      info!("Assigning index {} to {}", device_index, device.name());
                      let mut recv = device.get_event_receiver();
                      let device_added_message = DeviceAdded::new(
                        device_index,
                        &device.name(),
                        &device.message_attributes(),
                      );
                      device_map_clone.insert(device_index, device);
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
  // This uses a map to make sure we don't have 2 comm managers of the same type
  // register. Also means we can do lockless access since it's a Dashmap.
  comm_managers: Arc<DashMap<String, Box<dyn DeviceCommunicationManager>>>,
  devices: Arc<DashMap<u32, ButtplugDevice>>,
  sender: Sender<DeviceCommunicationEvent>,
}

unsafe impl Send for DeviceManager {
}

unsafe impl Sync for DeviceManager {
}

impl DeviceManager {
  pub fn new_with_options(
    event_sender: Sender<ButtplugServerMessage>,
    ping_receiver: Option<Receiver<()>>,
    allow_raw_messages: bool,
    device_config_json: &Option<String>,
    user_device_config_json: &Option<String>,
  ) -> Result<Self, ButtplugDeviceError> {
    let config = Arc::new(DeviceConfigurationManager::new_with_options(
      allow_raw_messages,
      device_config_json,
      user_device_config_json,
    )?);
    let (event_loop_fut, device_map, device_event_sender) =
      wait_for_manager_events(config, ping_receiver, event_sender);
    async_manager::spawn(event_loop_fut).unwrap();
    Ok(Self {
      sender: device_event_sender,
      devices: device_map,
      comm_managers: Arc::new(DashMap::new()),
    })
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
              device_name: dev.name(),
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

  pub fn add_comm_manager<T>(&self) -> Result<(), ButtplugServerStartupError>
  where
    T: 'static + DeviceCommunicationManager + DeviceCommunicationManagerCreator,
  {
    let mgr = T::new(self.sender.clone());
    if self.comm_managers.contains_key(mgr.name()) {
      return Err(ButtplugServerStartupError::DeviceManagerTypeAlreadyAdded(
        mgr.name().to_owned(),
      ));
    }
    let status = mgr.scanning_status();
    let sender = self.sender.clone();
    // TODO This could run out of order and possibly cause weird scanning finished bugs?
    async_manager::spawn(async move {
      sender
        .send(DeviceCommunicationEvent::DeviceManagerAdded(status))
        .await
        .unwrap();
    })
    .unwrap();
    self
      .comm_managers
      .insert(mgr.name().to_owned(), Box::new(mgr));
    Ok(())
  }

  pub fn add_test_comm_manager(
    &self,
  ) -> Result<TestDeviceCommunicationManagerHelper, ButtplugServerStartupError> {
    let mgr = TestDeviceCommunicationManager::new(self.sender.clone());
    if self.comm_managers.contains_key(mgr.name()) {
      return Err(ButtplugServerStartupError::DeviceManagerTypeAlreadyAdded(
        mgr.name().to_owned(),
      ));
    }
    let status = mgr.scanning_status();
    let sender = self.sender.clone();
    // TODO This could run out of order and possibly cause weird scanning finished bugs?
    async_manager::spawn(async move {
      sender
        .send(DeviceCommunicationEvent::DeviceManagerAdded(status))
        .await
        .unwrap();
    })
    .unwrap();
    let helper = mgr.helper();
    self
      .comm_managers
      .insert(mgr.name().to_owned(), Box::new(mgr));
    Ok(helper)
  }
}

impl Drop for DeviceManager {
  fn drop(&mut self) {
    info!("Dropping device manager!");
  }
}
