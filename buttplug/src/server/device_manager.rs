// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug Device Manager, manages Device Subtype (Platform/Communication bus
//! specific) Managers

use crate::{
    core::{
        errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
        messages::{
            self, ButtplugDeviceCommandMessageUnion, ButtplugDeviceManagerMessageUnion,
            ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageUnion, DeviceAdded,
            ScanningFinished, DeviceMessageInfo, DeviceList,
        },
    },
    device::device::{ButtplugDevice, ButtplugDeviceImplCreator},
};
use async_std::{
    prelude::StreamExt,
    sync::{channel, Receiver, Sender},
    task,
};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    convert::TryFrom,
    sync::{Arc, RwLock},
};

pub enum DeviceCommunicationEvent {
    // This event only means that a device has been found. The work still needs
    // to be done to make sure we can use it.
    DeviceFound(Box<dyn ButtplugDeviceImplCreator>),
    ScanningFinished,
}

// Storing this in a Vec<Box<dyn T>> causes a associated function issue due to
// the lack of new. Just create an extra trait for defining comm managers.
pub trait DeviceCommunicationManagerCreator: Sync + Send {
    fn new(sender: Sender<DeviceCommunicationEvent>) -> Self;
}

#[async_trait]
pub trait DeviceCommunicationManager: Sync + Send {
    async fn start_scanning(&mut self) -> Result<(), ButtplugError>;
    async fn stop_scanning(&mut self) -> Result<(), ButtplugError>;
    fn is_scanning(&mut self) -> bool;
    // Events happen via channel senders passed to the comm manager.
}

pub struct DeviceManager {
    comm_managers: Vec<Box<dyn DeviceCommunicationManager>>,
    devices: Arc<RwLock<HashMap<u32, ButtplugDevice>>>,
    sender: Sender<DeviceCommunicationEvent>,
}

unsafe impl Send for DeviceManager {}
unsafe impl Sync for DeviceManager {}

async fn wait_for_manager_events(
    mut receiver: Receiver<DeviceCommunicationEvent>,
    sender: Sender<ButtplugMessageUnion>,
    device_map: Arc<RwLock<HashMap<u32, ButtplugDevice>>>,
) {
    let mut device_index: u32 = 0;
    loop {
        match receiver.next().await {
            Some(event) => match event {
                DeviceCommunicationEvent::DeviceFound(device_creator) => {
                    match ButtplugDevice::try_create_device(device_creator).await {
                        Ok(option_dev) => match option_dev {
                            Some(device) => {
                                info!("Assigning index {} to {}", device_index, device.name());
                                sender
                                    .send(
                                        DeviceAdded::new(
                                            device_index,
                                            &device.name().to_owned(),
                                            &device.message_attributes(),
                                        )
                                            .into(),
                                    )
                                    .await;
                                device_map.write().unwrap().insert(device_index, device);
                                device_index += 1;
                            }
                            None => debug!("Device could not be matched to a protocol."),
                        },
                        Err(e) => error!("Device errored while trying to connect: {}", e),
                    }
                }
                DeviceCommunicationEvent::ScanningFinished => {
                    sender.send(ScanningFinished::default().into()).await;
                }
            },
            None => break,
        }
    }
}

impl DeviceManager {
    pub fn new(event_sender: Sender<ButtplugMessageUnion>) -> Self {
        let (sender, receiver) = channel(256);
        let map = Arc::new(RwLock::new(HashMap::new()));
        let map_clone = map.clone();
        let thread_sender = event_sender.clone();
        task::spawn(async move {
            wait_for_manager_events(receiver, thread_sender, map_clone).await;
        });
        Self {
            sender,
            devices: map,
            comm_managers: vec![],
        }
    }

    pub async fn start_scanning(&mut self) -> Result<(), ButtplugError> {
        // TODO This should error if we have no device managers
        for mgr in self.comm_managers.iter_mut() {
            mgr.start_scanning().await?;
        }
        Ok(())
    }

    pub async fn stop_scanning(&mut self) -> Result<(), ButtplugError> {
        // TODO This should error if we have no device managers
        for mgr in self.comm_managers.iter_mut() {
            mgr.stop_scanning().await?;
        }
        Ok(())
    }

    async fn parse_device_message(&mut self,
                                  device_msg: ButtplugDeviceCommandMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        let mut dev;
        match self
            .devices
            .read()
            .unwrap()
            .get(&device_msg.get_device_index())
        {
            Some(device) => {
                dev = device.clone();
            }
            None => {
                return Err(ButtplugDeviceError::new(&format!(
                    "No device with index {} available",
                    device_msg.get_device_index()
                )).into());
            }
        }
        // Note: Don't try moving this up into the Some branch of unlock/get for
        // the device array. We need to just copy the device out of that as
        // quickly as possible to release the lock, then actually parse the
        // message.
        //
        // TODO This should probably spawn or something
        dev.parse_message(&device_msg).await
    }

    async fn parse_device_manager_message(&mut self,
                                          manager_msg: ButtplugDeviceManagerMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        match manager_msg {
            ButtplugDeviceManagerMessageUnion::RequestDeviceList(msg) => {
                let devices = self
                    .devices
                    .read()
                    .unwrap()
                    .iter()
                    .map(|(id, device)|
                         DeviceMessageInfo {
                             device_index: *id,
                             device_name: device.name().to_string(),
                             device_messages: device.message_attributes(),
                         }
                    )
                    .collect();
                let mut device_list = DeviceList::new(devices);
                device_list.set_id(msg.get_id());
                Ok(device_list.into())
            }
            ButtplugDeviceManagerMessageUnion::StopAllDevices(msg) => {
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
        msg: ButtplugMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        // If this is a device command message, just route it directly to the
        // device.
        match ButtplugDeviceCommandMessageUnion::try_from(msg.clone()) {
            Ok(device_msg) => self.parse_device_message(device_msg).await,
            Err(_) => {
                match ButtplugDeviceManagerMessageUnion::try_from(msg.clone()) {
                    Ok(manager_msg) => self.parse_device_manager_message(manager_msg).await,
                    Err(_) => Err(ButtplugMessageError::new("Message type not handled by Device Manager").into())
                }
            }
        }
    }

    pub fn add_comm_manager<T>(&mut self)
    where
        T: 'static + DeviceCommunicationManager + DeviceCommunicationManagerCreator,
    {
        self.comm_managers
            .push(Box::new(T::new(self.sender.clone())));
    }
}

#[cfg(all(test, any(feature = "winrt-ble", feature = "linux-ble")))]
mod test {
    use super::DeviceManager;
    use crate::{
        core::messages::{ButtplugMessage, ButtplugMessageUnion, VibrateCmd, VibrateSubcommand, RequestDeviceList},
        server::comm_managers::btleplug::BtlePlugCommunicationManager,
    };
    use async_std::{prelude::StreamExt, sync::channel, task};
    use std::time::Duration;

    #[test]
    #[ignore]
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
                match dm
                    .parse_message(RequestDeviceList::default().into())
                    .await
                {
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
