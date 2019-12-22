use crate::{
    server::device_manager::DeviceCommunicationManager,
    core::errors::ButtplugError,
    devices::configuration_manager::{DeviceConfigurationManager, BluetoothLESpecifier, DeviceSpecifier},
};

use rumble::api::{UUID, Central, Peripheral, CentralEvent};
use async_trait::async_trait;
use async_std::{
    task,
    sync::channel,
    prelude::StreamExt,
};
#[cfg(feature = "winrt-ble")]
use rumble::winrtble::{manager::Manager, adapter::Adapter};
#[cfg(feature = "linux-ble")]
use rumble::bluez::manager::Manager;


struct RumbleBLECommunicationManager {
    manager: Manager,
}

impl RumbleBLECommunicationManager {
    pub fn new() -> Self {
        Self {
            manager: Manager::new(),
        }
    }

    #[cfg(feature = "winrt-ble")]
    fn get_central(&self) -> Adapter {
        self.manager.adapters().unwrap()
    }

    #[cfg(feature = "linux-ble")]
    fn get_central(&self) -> impl Central {
        let adapters = self.manager.adapters().unwrap();
        let adapter = adapters.into_iter().nth(0).unwrap();
        adapter.connect().unwrap()
    }
}

#[async_trait]
impl DeviceCommunicationManager for RumbleBLECommunicationManager {
    async fn start_scanning(&mut self) -> Result<(), ButtplugError> {
        println!("Getting adapter");
        // get the first bluetooth adapter

        let central = self.get_central();
        let device_mgr = DeviceConfigurationManager::load_from_internal();
        task::block_on(async move {
            let (sender, mut receiver) = channel(256);
            let on_event = move |event: CentralEvent| {
                match event {
                    CentralEvent::DeviceDiscovered(addr) => {
                        let s = sender.clone();
                        task::spawn(async move {
                            s.send(true).await;
                        });
                    },
                    _ => {}
                }
            };
            central.on_event(Box::new(on_event));
            println!("Scanning");
            central.start_scan().unwrap();
            let mut tried_names: Vec<String> = vec!();
            while receiver.next().await.unwrap() {
                for p in central.peripherals() {
                    if let Some(name) = p.properties().local_name {
                        if name.len() > 0 && !tried_names.contains(&name) {
                            tried_names.push(name.clone());
                            let ble_conf = BluetoothLESpecifier::new_from_device(&name);
                            error!("{}", name);
                            if device_mgr.find_protocol(&DeviceSpecifier::BluetoothLE(ble_conf)).is_some() {
                                error!("THIS IS A BUTTPLUG DEVICE");
                                let mut dev = RumbleBLEDeviceImpl::new(p);
                                dev.connect().unwrap();
                                error!("Done in connect!");
                            }
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

pub struct RumbleBLEDeviceImpl<T> where T: Peripheral {
    device: T
}

unsafe impl<T: Peripheral> Send for RumbleBLEDeviceImpl<T> {}
unsafe impl<T: Peripheral> Sync for RumbleBLEDeviceImpl<T> {}

impl<T: Peripheral> RumbleBLEDeviceImpl<T> {
    pub fn new(device: T) -> Self {
        Self {
            device
        }
    }

    pub fn connect(&mut self) -> Result<(), ButtplugError> {
        println!("Running connect!");
        self.device.connect().unwrap();
        println!("Discovering chars!");
        let chars = self.device.discover_characteristics().unwrap();
        println!("Getting chars!");
        //let chars = self.device.characteristics();
        println!("{:?}", chars);
        println!("Finding chars!");
        let mut id = [0x6e, 0x40, 0x00, 0x02, 0xb5, 0xa3, 0xf3, 0x93, 0xe0, 0xa9, 0xe5, 0x0e, 0x24, 0xdc, 0xca, 0x9e];
        // BLE uses little-endian addresses, and the library follows this. So we
        // have to flip our characteristic UUID.
        id.reverse();
        println!("{:?}", id);
        let chr = chars.into_iter().find(|c| { println!("{}", c); c.uuid == UUID::B128(id) }).unwrap();
        println!("{}", chr);
        let command = "Vibrate:20;".as_bytes();
        println!("Sending command!");
        self.device.command(&chr, &command).unwrap();
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::server::device_manager::DeviceCommunicationManager;
    use super::RumbleBLECommunicationManager;
    use async_std::task;
    use env_logger;

    #[test]
    pub fn test_rumble() {
        println!("Just trying to print anything?");
        let _ = env_logger::builder().is_test(true).try_init();
        task::block_on(async move {
            println!("Running?");
            let mut mgr = RumbleBLECommunicationManager::new();
            mgr.start_scanning().await;
        });
    }
}
