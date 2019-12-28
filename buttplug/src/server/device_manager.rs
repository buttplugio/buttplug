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
            ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageUnion, DeviceAdded, RawReadCmd,
            RawReading, RawWriteCmd,
        },
    },
    devices::{protocol::ButtplugProtocol, Endpoint},
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
    sync::{Arc, Mutex},
};

pub enum ButtplugProtocolRawMessage {
    RawWriteCmd(RawWriteCmd),
    RawReadCmd(RawReadCmd),
}

pub enum ButtplugDeviceResponseMessage {
    Ok(messages::Ok),
    Error(messages::Error),
    RawReading(RawReading),
}

pub enum ButtplugDeviceEvent {
    DeviceRemoved(),
    MessageEmitted(),
}

pub enum DeviceCommunicationEvent {
    DeviceAdded(ButtplugDevice),
    ScanningFinished,
}

pub struct ButtplugDevice {
    protocol: Box<dyn ButtplugProtocol>,
    device: Box<dyn DeviceImpl>,
}

impl ButtplugDevice {
    pub fn new(protocol: Box<dyn ButtplugProtocol>, device: Box<dyn DeviceImpl>) -> Self {
        Self { protocol, device }
    }

    pub fn name(&self) -> String {
        self.device.name()
    }

    pub async fn parse_message(
        &mut self,
        message: &ButtplugDeviceCommandMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        self.protocol.parse_message(&self.device, message).await
    }
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

#[async_trait]
pub trait DeviceImpl: Sync + Send {
    fn name(&self) -> String;
    fn address(&self) -> String;
    fn connected(&self) -> bool;
    fn endpoints(&self) -> Vec<Endpoint>;
    fn disconnect(&self);

    async fn read_value(&self, msg: &RawReadCmd) -> Result<RawReading, ButtplugError>;
    async fn write_value(&self, msg: &RawWriteCmd) -> Result<(), ButtplugError>;
}

pub struct DeviceManager {
    comm_managers: Vec<Box<dyn DeviceCommunicationManager>>,
    devices: Arc<Mutex<HashMap<u32, ButtplugDevice>>>,
    sender: Sender<DeviceCommunicationEvent>,
    event_sender: Sender<ButtplugMessageUnion>,
}

async fn wait_for_manager_events(
    mut receiver: Receiver<DeviceCommunicationEvent>,
    sender: Sender<ButtplugMessageUnion>,
    device_map: Arc<Mutex<HashMap<u32, ButtplugDevice>>>,
) {
    let mut device_index: u32 = 0;
    loop {
        match receiver.next().await.unwrap() {
            DeviceCommunicationEvent::DeviceAdded(device) => {
                info!("Assigning index {} to {}", device_index, device.name());
                // TODO Emit a DeviceAdded event here.
                sender
                    .send(DeviceAdded::new(device_index, &device.name(), &HashMap::new()).into())
                    .await;
                device_map.lock().unwrap().insert(device_index, device);
                device_index += 1;
            }
            DeviceCommunicationEvent::ScanningFinished => {
                // TODO Emit a ScanningFinished event here.
            }
        }
    }
}

impl DeviceManager {
    pub fn new(event_sender: Sender<ButtplugMessageUnion>) -> Self {
        let (sender, receiver) = channel(256);
        let map = Arc::new(Mutex::new(HashMap::new()));
        let map_clone = map.clone();
        let thread_sender = event_sender.clone();
        task::spawn(async move {
            wait_for_manager_events(receiver, thread_sender, map_clone).await;
        });
        Self {
            sender,
            devices: map,
            comm_managers: vec![],
            event_sender,
        }
    }

    pub async fn start_scanning(&mut self) -> Result<(), ButtplugError> {
        for mgr in self.comm_managers.iter_mut() {
            mgr.start_scanning().await;
        }
        Ok(())
    }

    pub async fn stop_scanning(&mut self) -> Result<(), ButtplugError> {
        for mgr in self.comm_managers.iter_mut() {
            mgr.stop_scanning().await;
        }
        Ok(())
    }

    pub async fn parse_message(
        &mut self,
        msg: ButtplugMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        // If this is a device command message, just route it directly to the
        // device.
        if let Ok(device_msg) = ButtplugDeviceCommandMessageUnion::try_from(msg.clone()) {
            // TODO This lock is going to mean only one device can process a
            // command at any time. That's bad. We should probably have the
            // HashMap be a RWLock that holds a Arc<Mutex<ButtplugDevice>>.
            if let Some(device) = self
                .devices
                .lock()
                .unwrap()
                .get_mut(&device_msg.get_device_index())
            {
                device.parse_message(&device_msg).await
            } else {
                Err(ButtplugError::ButtplugDeviceError(
                    ButtplugDeviceError::new(&format!(
                        "No device with index {} available",
                        device_msg.get_device_index()
                    )),
                ))
            }
        } else {
            if let Ok(manager_msg) = ButtplugDeviceManagerMessageUnion::try_from(msg.clone()) {
                match manager_msg {
                    ButtplugDeviceManagerMessageUnion::RequestDeviceList(msg) => {
                        Ok(messages::Ok::new(msg.get_id()).into())
                    }
                    ButtplugDeviceManagerMessageUnion::StopAllDevices(msg) => {
                        Ok(messages::Ok::new(msg.get_id()).into())
                    }
                    ButtplugDeviceManagerMessageUnion::StartScanning(msg) => {
                        self.start_scanning().await;
                        Ok(messages::Ok::new(msg.get_id()).into())
                    }
                    ButtplugDeviceManagerMessageUnion::StopScanning(msg) => {
                        self.stop_scanning().await;
                        Ok(messages::Ok::new(msg.get_id()).into())
                    }
                }
            } else {
                Err(ButtplugMessageError::new("Message type not handled by Device Manager").into())
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

#[cfg(all(test, any(feature="winrt-ble", feature="linux-ble")))]
mod test {
    use super::DeviceManager;
    use crate::{
        core::messages::{ButtplugMessageUnion, VibrateCmd, VibrateSubcommand},
        server::comm_managers::rumble_ble_comm_manager::RumbleBLECommunicationManager,
    };
    use async_std::{prelude::StreamExt, sync::channel, task};
    use std::time::Duration;

    #[test]
    pub fn test_device_manager_creation() {
        let _ = env_logger::builder().is_test(true).try_init();
        task::block_on(async {
            let (sender, mut receiver) = channel(256);
            let mut dm = DeviceManager::new(sender);
            dm.add_comm_manager::<RumbleBLECommunicationManager>();
            dm.start_scanning().await;
            if let ButtplugMessageUnion::DeviceAdded(msg) = receiver.next().await.unwrap() {
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
            task::sleep(Duration::from_secs(2)).await;
        });
    }
}
