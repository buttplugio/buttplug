use std::sync::mpsc::Sender;
use std::vec::Vec;
use std::collections::HashMap;
use messages::{Message, IncomingMessage};
mod trancevibe_wrapper;

#[derive(Serialize, Deserialize, Clone)]
pub struct DeviceInfo {
    pub device_name: String,
    pub device_id: String
}

pub struct DeviceManager {
    devices: Vec<DeviceInfo>,
    opened_devices: HashMap<u32, Sender<Message>>,
}

impl DeviceManager {
    pub fn new() -> DeviceManager {
        DeviceManager {
            devices: Vec::new(),
            opened_devices: HashMap::new()
        }
    }

    pub fn refresh_device_list(&self) {
    }

    pub fn get_device_list(&self) {
    }

    pub fn open_device(&self, device_id: u32) {
    }

    pub fn close_device(&self, device_id: u32) {
    }

    pub fn handle_message(&self, msg: &IncomingMessage) {
    }
}

