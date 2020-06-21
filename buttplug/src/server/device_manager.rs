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
use async_channel::{Receiver, Sender, bounded};
use dashmap::DashMap;
use futures::{FutureExt, StreamExt, future::{self, Future}};
use std::{convert::TryFrom, sync::{Arc, atomic::{AtomicU32, Ordering}}};

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
              main_device_index.store(main_device_index.load(Ordering::SeqCst) + 1, Ordering::SeqCst);
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
                      }).unwrap();
                      let device_added_message = DeviceAdded::new(
                        device_index,
                        device.name(),
                        &device.message_attributes(),
                      );
                      device_map_clone.insert(device_index, device);
                      // After that, we can send out to the server's event
                      // listeners to let them know a device has been added.
                      if server_sender_clone
                        .send(device_added_message.into())
                        .await
                        .is_err() {
                          error!("Server disappeared, exiting loop.");
                          return;
                        }
                    }
                    None => debug!("Device could not be matched to a protocol."),
                  },
                  Err(e) => error!("Device errored while trying to connect: {}", e),
                }
              }).unwrap();
            }
            DeviceCommunicationEvent::ScanningFinished => {
              if server_sender.send(ScanningFinished::default().into()).await.is_err() {
                error!("Server disappeared, exiting loop.");
                return;
              }
            }
          },
          None => break,
        },
        DeviceEvent::DeviceEvent(e) => match e {
          Some((idx, event)) => {
            if let ButtplugDeviceEvent::Removed = event {
              device_map.remove(&idx);
              if server_sender.send(DeviceRemoved::new(idx).into()).await.is_err() {
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
  comm_managers: Vec<Box<dyn DeviceCommunicationManager>>,
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
      comm_managers: vec![],
    }
  }

  fn start_scanning(&self, msg_id: u32) -> ButtplugServerResultFuture {
    if self.comm_managers.is_empty() {
      ButtplugUnknownError::new(
        "Cannot start scanning. Server has no device communication managers to scan with.",
      )
      .into()
    } else {
      let fut_vec: Vec<_> = self.comm_managers.iter().map(|mgr| mgr.start_scanning()).collect();
      Box::pin(async move {
        future::join_all(fut_vec).await;
        Ok(messages::Ok::new(msg_id).into())
      })
    }
  }

  fn stop_scanning(&self, msg_id: u32) -> ButtplugServerResultFuture {
    if self.comm_managers.is_empty() {
      ButtplugUnknownError::new(
        "Cannot start scanning. Server has no device communication managers to scan with.",
      )
      .into()
    } else {
      let fut_vec: Vec<_> = self.comm_managers.iter().map(|mgr| mgr.stop_scanning()).collect();
      Box::pin(async move {
        // TODO If stop_scanning fails anywhere, this will ignore it. We should maybe at least log?
        future::join_all(fut_vec).await;
        Ok(messages::Ok::new(msg_id).into())
      })
    }
  }

  fn stop_all_devices(&self, msg_id: u32) -> ButtplugServerResultFuture {
    let fut_vec: Vec<_> = self
      .devices
      .iter()
      .map(|dev| {
        let device = dev.value();
        device.parse_message(messages::StopDeviceCmd::new(1).into())
      })
      .collect();
    // TODO This could use some error reporting.
    Box::pin(async move {
      future::join_all(fut_vec).await;
      Ok(messages::Ok::new(msg_id).into())
    })
  }

  fn parse_device_message(
    &self,
    device_msg: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugServerResultFuture {
    match self.devices.get(&device_msg.get_device_index()) {
      Some(device) => device.parse_message(device_msg),
      None => ButtplugDeviceError::new(&format!(
        "No device with index {} available",
        device_msg.get_device_index()
      )).into(),
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
      ButtplugDeviceManagerMessageUnion::StopAllDevices(msg) => {
        self.stop_all_devices(msg.get_id())
      }
      ButtplugDeviceManagerMessageUnion::StartScanning(msg) => {
        self.start_scanning(msg.get_id())
      }
      ButtplugDeviceManagerMessageUnion::StopScanning(msg) => {
        self.stop_scanning(msg.get_id())
      }
    }
  }

  pub fn parse_message(
    &self,
    msg: ButtplugClientMessage,
  ) -> ButtplugServerResultFuture {
    // If this is a device command message, just route it directly to the
    // device.
    match ButtplugDeviceCommandMessageUnion::try_from(msg.clone()) {
      Ok(device_msg) => self.parse_device_message(device_msg),
      Err(_) => match ButtplugDeviceManagerMessageUnion::try_from(msg) {
        Ok(manager_msg) => self.parse_device_manager_message(manager_msg),
        Err(_) => {
          ButtplugMessageError::new("Message type not handled by Device Manager").into()
        }
      },
    }
  }

  pub fn add_comm_manager<T>(&mut self)
  where
    T: 'static + DeviceCommunicationManager + DeviceCommunicationManagerCreator,
  {
    self
      .comm_managers
      .push(Box::new(T::new(self.sender.clone())));
  }

  pub fn add_test_comm_manager(&mut self) -> TestDeviceCommunicationManagerHelper {
    let mgr = TestDeviceCommunicationManager::new(self.sender.clone());
    let helper = mgr.helper();
    self
      .comm_managers
      .push(Box::new(mgr));
    helper
  }
}

impl Drop for DeviceManager {
  fn drop(&mut self) {
    info!("Dropping device manager!");
  }
}

#[cfg(all(
  test,
  feature = "btleplug-manager"
))]
mod test {
  // TODO Rewrite this using modern api. Got stuck behind unused features for a while.
  /*
  use super::DeviceManager;
  use crate::{
    core::messages::{
      ButtplugMessage, ButtplugMessageUnion, RequestDeviceList, VibrateCmd, VibrateSubcommand,
    },
    server::comm_managers::btleplug::BtlePlugCommunicationManager,
    util::async_manager
  };
  use futures::StreamExt;
  use async_channel::bounded;
  use std::time::Duration;

  #[test]
  pub fn test_device_manager_creation() {
    async_manager::block_on(async {
      let (sender, mut receiver) = bounded(256);
      let mut dm = DeviceManager::new(sender);
      dm.add_comm_manager::<BtlePlugCommunicationManager>();
      dm.start_scanning().await;
      if let ButtplugMessageUnion::DeviceAdded(msg) = receiver.next().await.unwrap() {
        dm.stop_scanning().await;
        info!("{:?}", msg);
        info!("{:?}", msg.as_protocol_json());
        match dm.parse_message(RequestDeviceList::default().into()).await {
          Ok(msg) => info!("{:?}", msg),
          Err(e) => assert!(false, e.to_string()),
        }
        match dm
          .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
          .await
        {
          Ok(_) => info!("Message sent ok!"),
          Err(e) => assert!(false, e.to_string()),
        }
      } else {
        panic!("Did not get device added message!");
      }
      task::sleep(Duration::from_secs(10)).await;
    });
  }
  */
}
