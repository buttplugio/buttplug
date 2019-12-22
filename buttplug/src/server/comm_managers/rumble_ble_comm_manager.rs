use crate::{
    server::device_manager::{ DeviceCommunicationManager,
                              DeviceImpl,
                              ButtplugProtocolRawMessage,
                              ButtplugDeviceResponseMessage },
    core::{
        errors::{
            ButtplugError,
            ButtplugDeviceError,
        },
        messages::RawWriteCmd,
        messages::RawReadCmd,
        messages::RawReading,
    },
    devices::{
        Endpoint,
        configuration_manager::{DeviceConfigurationManager,
                                BluetoothLESpecifier,
                                DeviceSpecifier,
                                ProtocolDefinition},
    }
};
use std::collections::HashMap;
use uuid;
use rumble::api::{UUID, Central, Peripheral, CentralEvent, Characteristic};
use async_trait::async_trait;
use async_std::{
    task,
    sync::{channel, Sender, Receiver},
    prelude::StreamExt,
};
#[cfg(feature = "winrt-ble")]
use rumble::winrtble::{manager::Manager, adapter::Adapter};
#[cfg(feature = "linux-ble")]
use rumble::bluez::{manager::Manager, adapter::ConnectedAdapter};

struct RumbleBLECommunicationManager {
    manager: Manager,
}

#[cfg(feature = "win-ble")]
impl RumbleBLECommunicationManager {
    pub fn new() -> Self {
        Self {
            manager: Manager::new(),
        }
    }

    fn get_central(&self) -> Adapter {
        self.manager.adapters().unwrap()
    }
}

#[cfg(feature = "linux-ble")]
impl RumbleBLECommunicationManager {
    pub fn new() -> Self {
        Self {
            manager: Manager::new().unwrap(),
        }
    }

    fn get_central(&self) -> ConnectedAdapter {
        let adapters = self.manager.adapters().unwrap();
        let adapter = adapters.into_iter().nth(0).unwrap();
        adapter.connect().unwrap()
    }
}

#[async_trait]
impl DeviceCommunicationManager for RumbleBLECommunicationManager {
    async fn start_scanning(&mut self) -> Result<(), ButtplugError> {
        // get the first bluetooth adapter
        debug!("Bringing up adapter.");
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
            info!("Starting scan.");
            central.start_scan().unwrap();
            let mut tried_names: Vec<String> = vec!();
            // This needs a way to cancel when we call stop_scanning.
            while receiver.next().await.unwrap() {
                for p in central.peripherals() {
                    if let Some(name) = p.properties().local_name {
                        debug!("Found BLE device {}", name);
                        // Names are the only way we really have to test devices
                        // at the moment. Most devices don't send services on
                        // advertisement.
                        if name.len() > 0 && !tried_names.contains(&name) {
                            tried_names.push(name.clone());
                            let ble_conf = BluetoothLESpecifier::new_from_device(&name);
                            if let Some(protocol) = device_mgr.find_protocol(&DeviceSpecifier::BluetoothLE(ble_conf)) {
                                info!("Found Buttplug Device {}", name);
                                let mut dev = RumbleBLEDeviceImpl::new(p);
                                dev.connect(&protocol).unwrap();
                                //dev.write_value(&RawWriteCmd::new(0, Endpoint::Tx, "Vibrate:20;".as_bytes().to_vec(), false)).await;
                            } else {
                                info!("Device {} is not recognized as a Buttplug Device.", name);
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
    device: T,
    endpoints: HashMap<Endpoint, Characteristic>
}

unsafe impl<T: Peripheral> Send for RumbleBLEDeviceImpl<T> {}
unsafe impl<T: Peripheral> Sync for RumbleBLEDeviceImpl<T> {}

fn uuid_to_rumble(uuid: &uuid::Uuid) -> UUID {
    let mut rumble_uuid = uuid.as_bytes().clone();
    rumble_uuid.reverse();
    UUID::B128(rumble_uuid)
}

impl<T: Peripheral> RumbleBLEDeviceImpl<T> {
    pub fn new(device: T) -> Self {
        Self {
            device,
            endpoints: HashMap::new()
        }
    }

    pub fn connect(&mut self, protocol: &ProtocolDefinition) -> Result<(), ButtplugError> {
        if let Some(ref proto) = protocol.btle {
            self.device.connect().unwrap();
            let chars = self.device.discover_characteristics().unwrap();
            for proto_service in proto.services.values() {
                info!("Searching services");
                for (chr_name, chr_uuid) in proto_service.into_iter() {
                    let chr = chars.iter().find(|c| { c.uuid == uuid_to_rumble(chr_uuid) });
                    if chr.is_some() {
                        info!("Found valid characteristic {}", chr_uuid);
                        self.endpoints.insert(*chr_name, chr.unwrap().clone());
                    }
                }
            }
            // TODO This should fail if we don't find any usable characteristics.
            Ok(())
        } else {
            panic!("Got a protocol with no Bluetooth Definition!");
        }
    }

}

#[async_trait]
impl<T: Peripheral> DeviceImpl for RumbleBLEDeviceImpl<T> {
    fn name(&self) -> String {
        self.device.properties().local_name.unwrap()
    }

    fn address(&self) -> String {
        self.device.properties().address.to_string()
    }
    fn connected(&self) -> bool {
        true
    }
    fn endpoints(&self) -> Vec<Endpoint> {
         self.endpoints.keys().map(|v| v.clone()).collect::<Vec<Endpoint>>()
    }
    fn disconnect(&self) {
        todo!("implement disconnect");
    }
    fn set_channel(&mut self, receiver: Receiver<ButtplugProtocolRawMessage>, sender: Sender<ButtplugDeviceResponseMessage>) {
        todo!("implement set channel");
    }
   async fn write_value(&self, msg: &RawWriteCmd) -> Result<(), ButtplugError> {
        match self.endpoints.get(&msg.endpoint) {
            Some(chr) => {
                self.device.command(&chr, &msg.data).unwrap();
                Ok(())
            },
            None => Err(ButtplugError::ButtplugDeviceError(ButtplugDeviceError::new(&format!("Device does not contain an endpoint named {}",msg.endpoint).to_owned())))
        }
    }

    async fn read_value(&self, msg: &RawReadCmd) -> Result<RawReading, ButtplugError> {
        Ok(RawReading::new(0, msg.endpoint, vec!()))
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
        let _ = env_logger::builder().is_test(true).try_init();
        task::block_on(async move {
            let mut mgr = RumbleBLECommunicationManager::new();
            mgr.start_scanning().await;
        });
    }
}
