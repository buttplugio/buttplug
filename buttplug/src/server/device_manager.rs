// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug Device Manager, manages Device Subtype (Platform/Communication bus
//! specific) Managers

use super::comm_managers::{
  DeviceCommunicationEvent,
  DeviceCommunicationManager,
  DeviceCommunicationManagerCreator,
};
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError, ButtplugUnknownError},
    messages::{
      self,
      ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceManagerMessageUnion,
      ButtplugDeviceMessage,
      ButtplugInMessage,
      ButtplugMessage,
      ButtplugOutMessage,
      DeviceAdded,
      DeviceList,
      DeviceMessageInfo,
      DeviceRemoved,
      ScanningFinished,
    },
  },
  device::device::{ButtplugDevice, ButtplugDeviceEvent},
  test::{TestDeviceCommunicationManager, TestDeviceImplCreator},
};
use async_std::{
  prelude::{FutureExt, StreamExt},
  sync::{channel, Arc, Mutex, Receiver, RwLock, Sender},
  task,
};

use std::{collections::HashMap, convert::TryFrom};

enum DeviceEvent {
  DeviceCommunicationEvent(Option<DeviceCommunicationEvent>),
  DeviceEvent(Option<(u32, ButtplugDeviceEvent)>),
  PingTimeout,
}

async fn wait_for_manager_events(
  mut device_comm_receiver: Receiver<DeviceCommunicationEvent>,
  ping_receiver: Option<Receiver<bool>>,
  sender: Sender<ButtplugOutMessage>,
  device_map: Arc<RwLock<HashMap<u32, ButtplugDevice>>>,
) {
  let mut device_index: u32 = 0;
  let (device_event_sender, mut device_event_receiver) = channel::<(u32, ButtplugDeviceEvent)>(256);
  loop {
    let recv_fut =
      async { DeviceEvent::DeviceCommunicationEvent(device_comm_receiver.next().await) };

    let device_event_fut = async { DeviceEvent::DeviceEvent(device_event_receiver.next().await) };

    let ping_fut = async {
      if let Some(recv) = &ping_receiver {
        recv.recv().await;
      } else {
        futures::future::pending::<bool>().await;
      }
      // If the ping receiver ever gets anything, we've pinged out, so
      // just stop everything and exit.
      DeviceEvent::PingTimeout
    };

    let race_fut = recv_fut.race(device_event_fut).race(ping_fut);

    match race_fut.await {
      DeviceEvent::DeviceCommunicationEvent(e) => match e {
        Some(event) => match event {
          DeviceCommunicationEvent::DeviceFound(device_creator) => {
            let device_event_sender_clone = device_event_sender.clone();
            let sender_sender_clone = sender.clone();
            let device_map_clone = device_map.clone();
            task::spawn(async move {
              match ButtplugDevice::try_create_device(device_creator).await {
                Ok(option_dev) => match option_dev {
                  Some(device) => {
                    info!("Assigning index {} to {}", device_index, device.name());
                    let mut recv = device.get_event_receiver();
                    let sender_clone = device_event_sender_clone.clone();
                    let idx_clone = device_index.clone();
                    task::spawn(async move {
                      loop {
                        match recv.next().await {
                          Some(e) => sender_clone.send((idx_clone, e)).await,
                          None => break,
                        }
                      }
                    });
                    sender_sender_clone
                      .send(
                        DeviceAdded::new(
                          device_index,
                          &device.name().to_owned(),
                          &device.message_attributes(),
                        )
                        .into(),
                      )
                      .await;
                    device_map_clone.write().await.insert(device_index, device);
                    device_index += 1;
                  }
                  None => debug!("Device could not be matched to a protocol."),
                },
                Err(e) => error!("Device errored while trying to connect: {}", e),
              }
            });
          }
          DeviceCommunicationEvent::ScanningFinished => {
            sender.send(ScanningFinished::default().into()).await;
          }
        },
        None => break,
      },
      DeviceEvent::DeviceEvent(e) => match e {
        Some((idx, event)) => {
          match event {
            ButtplugDeviceEvent::Removed => {
              let mut map = device_map.write().await;
              map.remove(&idx);
              sender.send(DeviceRemoved::new(idx).into()).await;
            }
            _ => {}
          }
          info!("Got device event: {:?}", event);
        }
        None => break,
      },
      DeviceEvent::PingTimeout => {
        // TODO This should be done in parallel, versus waiting for every device
        // to stop in order.
        error!("Pinged out, stopping devices");
        for (_, ref mut device) in device_map.write().await.iter_mut() {
          // Device index doesn't matter here, since we're sending the
          // message directly to the device itself.
          if let Err(e) = device
            .parse_message(&messages::StopDeviceCmd::new(1).into())
            .await
          {
            error!(
              "Error stopping device {} on ping timeout: {}",
              device.name(),
              e
            );
          }
        }
        break;
      }
    }
  }
}

pub struct DeviceManager {
  comm_managers: Vec<Box<dyn DeviceCommunicationManager>>,
  devices: Arc<RwLock<HashMap<u32, ButtplugDevice>>>,
  sender: Sender<DeviceCommunicationEvent>,
}

unsafe impl Send for DeviceManager {
}

unsafe impl Sync for DeviceManager {
}

impl DeviceManager {
  pub fn new(
    event_sender: Sender<ButtplugOutMessage>,
    ping_receiver: Option<Receiver<bool>>,
  ) -> Self {
    let (sender, receiver) = channel(256);
    let map = Arc::new(RwLock::new(HashMap::new()));
    let map_clone = map.clone();
    let thread_sender = event_sender.clone();
    task::spawn(async move {
      wait_for_manager_events(receiver, ping_receiver, thread_sender, map_clone).await;
    });
    Self {
      sender,
      devices: map,
      comm_managers: vec![],
    }
  }

  async fn start_scanning(&mut self) -> Result<(), ButtplugError> {
    if self.comm_managers.is_empty() {
      Err(
        ButtplugUnknownError::new(
          "Cannot start scanning. Server has no device communication managers to scan with.",
        )
        .into(),
      )
    } else {
      for mgr in self.comm_managers.iter_mut() {
        mgr.start_scanning().await?;
      }
      Ok(())
    }
  }

  async fn stop_scanning(&mut self) -> Result<(), ButtplugError> {
    if self.comm_managers.is_empty() {
      Err(
        ButtplugUnknownError::new(
          "Cannot stop scanning. Server has no device communication managers to scan with.",
        )
        .into(),
      )
    } else {
      for mgr in self.comm_managers.iter_mut() {
        mgr.stop_scanning().await?;
      }
      Ok(())
    }
  }

  async fn stop_all_devices(&mut self) -> Result<(), ButtplugError> {
    let devices_ids: Vec<u32> = self
      .devices
      .read()
      .await
      .keys()
      .map(|id| id.clone())
      .collect();
    // TODO This should be done in parallel, versus waiting for every device
    // to stop in order.
    for id in devices_ids {
      self
        .parse_device_message(messages::StopDeviceCmd::new(id).into())
        .await?;
    }
    Ok(())
  }

  async fn parse_device_message(
    &self,
    device_msg: ButtplugDeviceCommandMessageUnion,
  ) -> Result<ButtplugOutMessage, ButtplugError> {
    let mut dev;
    match self
      .devices
      .read()
      .await
      .get(&device_msg.get_device_index())
    {
      Some(device) => {
        dev = device.clone();
      }
      None => {
        return Err(
          ButtplugDeviceError::new(&format!(
            "No device with index {} available",
            device_msg.get_device_index()
          ))
          .into(),
        );
      }
    }
    // Note: Don't try moving this up into the Some branch of unlock/get for
    // the device array. We need to just copy the device out of that as
    // quickly as possible to release the lock, then actually parse the
    // message.
    //
    // TODO This should spawn so we're not blocking our receiver.
    dev.parse_message(&device_msg).await
  }

  async fn parse_device_manager_message(
    &mut self,
    manager_msg: ButtplugDeviceManagerMessageUnion,
  ) -> Result<ButtplugOutMessage, ButtplugError> {
    match manager_msg {
      ButtplugDeviceManagerMessageUnion::RequestDeviceList(msg) => {
        let devices = self
          .devices
          .read()
          .await
          .iter()
          .map(|(id, device)| DeviceMessageInfo {
            device_index: *id,
            device_name: device.name().to_string(),
            device_messages: device.message_attributes(),
          })
          .collect();
        let mut device_list = DeviceList::new(devices);
        device_list.set_id(msg.get_id());
        Ok(device_list.into())
      }
      ButtplugDeviceManagerMessageUnion::StopAllDevices(msg) => {
        self.stop_all_devices().await?;
        Ok(messages::Ok::new(msg.get_id()).into())
      }
      ButtplugDeviceManagerMessageUnion::StartScanning(msg) => {
        self.start_scanning().await?;
        Ok(messages::Ok::new(msg.get_id()).into())
      }
      ButtplugDeviceManagerMessageUnion::StopScanning(msg) => {
        self.stop_scanning().await?;
        Ok(messages::Ok::new(msg.get_id()).into())
      }
    }
  }

  pub async fn parse_message(
    &mut self,
    msg: ButtplugInMessage,
  ) -> Result<ButtplugOutMessage, ButtplugError> {
    // If this is a device command message, just route it directly to the
    // device.
    match ButtplugDeviceCommandMessageUnion::try_from(msg.clone()) {
      Ok(device_msg) => self.parse_device_message(device_msg).await,
      Err(_) => match ButtplugDeviceManagerMessageUnion::try_from(msg.clone()) {
        Ok(manager_msg) => self.parse_device_manager_message(manager_msg).await,
        Err(_) => {
          Err(ButtplugMessageError::new("Message type not handled by Device Manager").into())
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

  pub fn add_test_comm_manager(&mut self) -> Arc<Mutex<Vec<Box<TestDeviceImplCreator>>>> {
    let mgr = TestDeviceCommunicationManager::new(self.sender.clone());
    let devices = mgr.get_devices_clone();
    self.comm_managers.push(Box::new(mgr));
    devices
  }
}

impl Drop for DeviceManager {
  fn drop(&mut self) {
    info!("Dropping device manager!");
  }
}

#[cfg(all(
  test,
  any(
    feature = "winrt-ble",
    feature = "linux-ble",
    feature = "corebluetooth-ble"
  )
))]
mod test {
  use super::DeviceManager;
  use crate::{
    core::messages::{
      ButtplugMessage,
      ButtplugMessageUnion,
      RequestDeviceList,
      VibrateCmd,
      VibrateSubcommand,
    },
    server::comm_managers::btleplug::BtlePlugCommunicationManager,
  };
  use async_std::{prelude::StreamExt, sync::channel, task};
  use std::time::Duration;

  #[test]
  pub fn test_device_manager_creation() {
    let _ = env_logger::builder().is_test(true).try_init();
    task::block_on(async {
      let (sender, mut receiver) = channel(256);
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
}
