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
    errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError, ButtplugUnknownError},
    messages::{
      self, ButtplugClientMessage, ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceManagerMessageUnion, ButtplugDeviceMessage, ButtplugMessage,
      ButtplugServerMessage, DeviceAdded, DeviceList, DeviceMessageInfo, DeviceRemoved,
      ScanningFinished,
    },
    ButtplugResultFuture,
  },
  device::{ButtplugDevice, ButtplugDeviceEvent, ButtplugDeviceResultFuture},
  test::{TestDeviceCommunicationManager, TestDeviceImplCreator},
};
use async_std::{
  prelude::{FutureExt, StreamExt},
  sync::{channel, Receiver, Sender, Arc, Mutex},
  task,
};
use evmap::{self, ReadHandle};
use futures::future::Future;
use std::convert::TryFrom;

enum DeviceEvent {
  DeviceCommunicationEvent(Option<DeviceCommunicationEvent>),
  DeviceEvent(Option<(u32, ButtplugDeviceEvent)>),
  PingTimeout,
}

fn wait_for_manager_events(
  ping_receiver: Option<Receiver<bool>>,
  server_sender: Sender<ButtplugServerMessage>,
) -> (
  impl Future<Output = ()>,
  ReadHandle<u32, ButtplugDevice>,
  Sender<DeviceCommunicationEvent>,
) {
  let mut device_index: u32 = 0;
  let (device_event_sender, mut device_event_receiver) = channel::<(u32, ButtplugDeviceEvent)>(256);
  let (device_map_reader, mut device_map_writer) = evmap::new::<u32, ButtplugDevice>();
  // Refresh ASAP just in case we ping out before getting any devices.
  device_map_writer.refresh();
  let (device_comm_sender, mut device_comm_receiver) = channel(256);
  // Used for feeding devices back to ourselves in the loop.
  let device_comm_sender_internal = device_comm_sender.clone();
  let device_map_reader_internal = device_map_reader.clone();
  let event_loop = async move {
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
              let server_sender_clone = server_sender.clone();
              let device_comm_sender_internal_clone = device_comm_sender_internal.clone();
              task::spawn(async move {
                match ButtplugDevice::try_create_device(device_creator).await {
                  Ok(option_dev) => match option_dev {
                    Some(device) => {
                      info!("Assigning index {} to {}", device_index, device.name());
                      let mut recv = device.get_event_receiver();
                      let sender_clone = device_event_sender_clone.clone();
                      let idx_clone = device_index;
                      task::spawn(async move {
                        while let Some(e) = recv.next().await {
                          sender_clone.send((idx_clone, e)).await;
                        }
                      });
                      let device_added_message = DeviceAdded::new(
                        device_index,
                        device.name(),
                        &device.message_attributes(),
                      );
                      server_sender_clone
                        .send(device_added_message.into())
                        .await;
                      device_comm_sender_internal_clone
                        .send(DeviceCommunicationEvent::DeviceConnected((
                          device_index,
                          device,
                        )))
                        .await;
                      device_index += 1;
                    }
                    None => debug!("Device could not be matched to a protocol."),
                  },
                  Err(e) => error!("Device errored while trying to connect: {}", e),
                }
              });
            }
            DeviceCommunicationEvent::DeviceConnected((id, device)) => {
              device_map_writer.insert(id, device);
              device_map_writer.refresh();
            }
            DeviceCommunicationEvent::ScanningFinished => {
              server_sender.send(ScanningFinished::default().into()).await;
            }
          },
          None => break,
        },
        DeviceEvent::DeviceEvent(e) => match e {
          Some((idx, event)) => {
            if let ButtplugDeviceEvent::Removed = event {
              device_map_writer.empty(idx);
              server_sender.send(DeviceRemoved::new(idx).into()).await;
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
          let fut_vec: Vec<_> = device_map_reader_internal
            .read()
            .unwrap()
            .iter()
            .map(|(_, dev)| {
              let device = dev.get_one().unwrap();
              device.parse_message(&messages::StopDeviceCmd::new(1).into())
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
  (event_loop, device_map_reader, device_comm_sender)
}

pub struct DeviceManager {
  comm_managers: Vec<Box<dyn DeviceCommunicationManager>>,
  devices: ReadHandle<u32, ButtplugDevice>,
  sender: Sender<DeviceCommunicationEvent>,
}

unsafe impl Send for DeviceManager {}

unsafe impl Sync for DeviceManager {}

impl DeviceManager {
  pub fn new(
    event_sender: Sender<ButtplugServerMessage>,
    ping_receiver: Option<Receiver<bool>>,
  ) -> Self {
    let (event_loop_fut, device_map_reader, device_event_sender) =
      wait_for_manager_events(ping_receiver, event_sender);
    task::spawn(async move {
      event_loop_fut.await;
    });
    Self {
      sender: device_event_sender,
      devices: device_map_reader,
      comm_managers: vec![],
    }
  }

  fn start_scanning(&self) -> ButtplugResultFuture {
    if self.comm_managers.is_empty() {
      ButtplugUnknownError::new(
        "Cannot start scanning. Server has no device communication managers to scan with.",
      )
      .into()
    } else {
      let mut fut_vec = vec![];
      for mgr in self.comm_managers.iter() {
        fut_vec.push(mgr.start_scanning());
      }
      Box::pin(async {
        for fut in fut_vec {
          fut.await?;
        }
        Ok(())
      })
    }
  }

  fn stop_scanning(&self) -> ButtplugResultFuture {
    if self.comm_managers.is_empty() {
      ButtplugUnknownError::new(
        "Cannot start scanning. Server has no device communication managers to scan with.",
      )
      .into()
    } else {
      let mut fut_vec = vec![];
      for mgr in self.comm_managers.iter() {
        fut_vec.push(mgr.stop_scanning());
      }
      Box::pin(async {
        for fut in fut_vec {
          fut.await?;
        }
        Ok(())
      })
    }
  }

  fn stop_all_devices(&self) -> ButtplugResultFuture {
    let fut_vec: Vec<_> = self
      .devices
      .read()
      .unwrap()
      .iter()
      .map(|(_, dev)| {
        let device = dev.get_one().unwrap();
        device.parse_message(&messages::StopDeviceCmd::new(1).into())
      })
      .collect();
    // TODO This could use some error reporting.
    Box::pin(async {
      for fut in fut_vec {
        if let Err(e) = fut.await {
          error!("{:?}", e);
        }
      }
      Ok(())
    })
  }

  fn parse_device_message(
    &self,
    device_msg: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugDeviceResultFuture {
    match self.devices.get_one(&device_msg.get_device_index()) {
      Some(device) => device.parse_message(&device_msg),
      None => ButtplugDeviceError::new(&format!(
        "No device with index {} available",
        device_msg.get_device_index()
      )).into(),
    }
  }

  async fn parse_device_manager_message(
    &mut self,
    manager_msg: ButtplugDeviceManagerMessageUnion,
  ) -> Result<ButtplugServerMessage, ButtplugError> {
    match manager_msg {
      ButtplugDeviceManagerMessageUnion::RequestDeviceList(msg) => {
        let devices = self
          .devices
          .read()
          .unwrap()
          .iter()
          .map(|(id, device)| {
            let dev = device.get_one().unwrap();
            DeviceMessageInfo {
              device_index: *id,
              device_name: dev.name().to_string(),
              device_messages: dev.message_attributes(),
            }
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
    msg: ButtplugClientMessage,
  ) -> Result<ButtplugServerMessage, ButtplugError> {
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

  pub fn add_test_comm_manager(&mut self) -> Arc<Mutex<Vec<TestDeviceImplCreator>>> {
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
      ButtplugMessage, ButtplugMessageUnion, RequestDeviceList, VibrateCmd, VibrateSubcommand,
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
