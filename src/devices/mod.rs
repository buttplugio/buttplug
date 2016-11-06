use std::thread;
use std::cell::{Cell, RefCell};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::vec::Vec;
use std::collections::HashMap;
use messages;
use messages::{Message, IncomingMessage};
mod trancevibe_wrapper;
mod fake_device_wrapper;

#[derive(Clone)]
pub struct DeviceInfo {
    pub name: String,
    // TODO This should be a string
    pub id: u32,
    pub path: String,
    pub broadcast: bool,
    pub loop_closure: Arc<(Fn(String, Receiver<IncomingMessage>) + Send + Sync)>
}

pub struct DeviceThread {
    pub handle: JoinHandle<()>,
    pub channel: Sender<IncomingMessage>
}

pub struct DeviceManager {
    devices: HashMap<u32, DeviceInfo>,
    opened_devices: HashMap<u32, DeviceThread>,
}

impl DeviceManager {
    pub fn new() -> DeviceManager {
        let mut dm = HashMap::new();
        let d = fake_device_wrapper::discover_devices();
        for dev in d {
            dm.insert(dev.id.clone(), dev.clone());
        }
        DeviceManager {
            devices: dm,
            opened_devices: HashMap::new()
        }
    }

    pub fn refresh_device_list(&self) {
    }

    pub fn get_device_list(&self) {
    }

    fn open_device(&mut self, device_id: u32, msg: IncomingMessage) {
        warn!("Got claim device message!");
        if !self.devices.contains_key(&device_id) {
            (msg.callback)(messages::Error::as_message("Device id not found!".to_string()));
        }
        let mut dev : &mut DeviceInfo = self.devices.get_mut(&device_id).unwrap();
        let (tx, rx) = channel();
        let lp = dev.loop_closure.clone();
        let path = dev.path.clone();
        warn!("about to start thread!");
        let thr = thread::spawn(move || {
            warn!("Starting thread!");
            (lp)(path, rx);
        });
        //let i = msg.clone();
        //tx.send(msg);
        self.opened_devices.insert(device_id, DeviceThread {
            handle: thr,
            channel: tx
        });
    }

    pub fn handle_message(&mut self, msg: IncomingMessage) {
        // Since we have to destructure into the message itself then possibly
        // pass it on, clone the message data up front to match on. It's tiny
        // anyways.
        let match_msg = msg.msg.clone();
        match match_msg {
            messages::Message::Device(id, d) => {
                match d {
                    messages::Device::ClaimDevice(_) => self.open_device(id, msg),
                    _ => {
                        if !self.opened_devices.contains_key(&id) {
                            (msg.callback)(messages::Error::as_message("Device not open!".to_string()));
                            return;
                        }
                        self.opened_devices.get(&id).unwrap().channel.send(msg);
                    }
                }
            },
            _ => {}
        }
    }
}
