mod rumble_internal;
mod rumble_device_impl;

use crate::{
    core::{
        errors::ButtplugError,
    },
    server::device_manager::{
        DeviceCommunicationEvent, DeviceCommunicationManager, DeviceCommunicationManagerCreator,
    },
};
use async_std::{
    prelude::StreamExt,
    sync::{channel, Sender},
    task,
};
use rumble_device_impl::RumbleBLEDeviceImplCreator;
use async_trait::async_trait;
use rumble::api::{Central, CentralEvent, Peripheral};
#[cfg(feature = "linux-ble")]
use rumble::bluez::{adapter::ConnectedAdapter, manager::Manager};
#[cfg(feature = "winrt-ble")]
use rumble::winrtble::{adapter::Adapter, manager::Manager};

pub struct RumbleBLECommunicationManager {
    manager: Manager,
    device_sender: Sender<DeviceCommunicationEvent>,
}

#[cfg(feature = "winrt-ble")]
impl RumbleBLECommunicationManager {
    fn get_central(&self) -> Adapter {
        self.manager.adapters().unwrap()
    }
}

#[cfg(feature = "linux-ble")]
impl RumbleBLECommunicationManager {
    fn get_central(&self) -> ConnectedAdapter {
        let adapters = self.manager.adapters().unwrap();
        let adapter = adapters.into_iter().nth(0).unwrap();
        adapter.connect().unwrap()
    }
}

impl DeviceCommunicationManagerCreator for RumbleBLECommunicationManager {
    #[cfg(feature = "winrt-ble")]
    fn new(device_sender: Sender<DeviceCommunicationEvent>) -> Self {
        Self {
            manager: Manager::new(),
            device_sender,
        }
    }

    #[cfg(feature = "linux-ble")]
    fn new(device_sender: Sender<DeviceCommunicationEvent>) -> Self {
        Self {
            manager: Manager::new().unwrap(),
            device_sender,
        }
    }
}

#[async_trait]
impl DeviceCommunicationManager for RumbleBLECommunicationManager {
    async fn start_scanning(&mut self) -> Result<(), ButtplugError> {
        // get the first bluetooth adapter
        debug!("Bringing up adapter.");
        let central = self.get_central();
        let device_sender = self.device_sender.clone();
        task::spawn(async move {
            let (sender, mut receiver) = channel(256);
            let on_event = move |event: CentralEvent| match event {
                CentralEvent::DeviceDiscovered(_) => {
                    let s = sender.clone();
                    task::spawn(async move {
                        s.send(true).await;
                    });
                }
                _ => {}
            };
            central.on_event(Box::new(on_event));
            info!("Starting scan.");
            central.start_scan().unwrap();
            // TODO This should be "tried addresses" probably. Otherwise if we
            // want to connect, say, 2 launches, we're going to have a Bad Time.
            let mut tried_names: Vec<String> = vec![];
            // This needs a way to cancel when we call stop_scanning.
            while receiver.next().await.unwrap() {
                for p in central.peripherals() {
                    // If a device has no discernable name, we can't do anything
                    // with it, just ignore it.
                    //
                    // TODO Should probably at least log this and add it to the
                    // tried_addresses thing, once that exists.
                    if let Some(name) = p.properties().local_name {
                        debug!("Found BLE device {}", name);
                        // Names are the only way we really have to test devices
                        // at the moment. Most devices don't send services on
                        // advertisement.
                        if name.len() > 0 && !tried_names.contains(&name) {
                            tried_names.push(name.clone());
                            let device_creator = Box::new(RumbleBLEDeviceImplCreator::new(p));
                            device_sender
                                .send(DeviceCommunicationEvent::DeviceFound(device_creator))
                                .await;
                        }
                    }
                }
            }
        });
        Ok(())
    }

    async fn stop_scanning(&mut self) -> Result<(), ButtplugError> {
        Ok(())
    }

    fn is_scanning(&mut self) -> bool {
        false
    }
}

#[cfg(all(test, any(feature = "winrt-ble", feature = "linux-ble")))]
mod test {
    use super::RumbleBLECommunicationManager;
    use crate::{
        core::messages::{ButtplugMessageUnion, VibrateCmd, VibrateSubcommand},
        server::device_manager::{
            DeviceCommunicationEvent, DeviceCommunicationManager, DeviceCommunicationManagerCreator,
        },
    };
    use async_std::{prelude::StreamExt, sync::channel, task};
    use env_logger;

    #[test]
    #[ignore]
    pub fn test_rumble() {
        let _ = env_logger::builder().is_test(true).try_init();
        task::block_on(async move {
            let (sender, mut receiver) = channel(256);
            let mut mgr = RumbleBLECommunicationManager::new(sender);
            mgr.start_scanning().await;
            loop {
                match receiver.next().await.unwrap() {
                    DeviceCommunicationEvent::DeviceFound(mut device) => {
                        info!("Got device!");
                        info!("Sending message!");
                        // TODO since we don't return full devices as this point
                        // anymore, we need to find some other way to test this.
                        //
                        // match device
                        //     .parse_message(
                        //         &VibrateCmd::new(1, vec![VibrateSubcommand::new(0, 0.5)]).into(),
                        //     )
                        //     .await
                        // {
                        //     Ok(msg) => match msg {
                        //         ButtplugMessageUnion::Ok(_) => info!("Returned Ok"),
                        //         _ => info!("Returned something other than ok"),
                        //     },
                        //     Err(_) => {
                        //         assert!(false, "Error returned from parse message");
                        //     }
                        // }
                    }
                    _ => assert!(false, "Shouldn't get other message types!"),
                }
            }
        });
    }
}
