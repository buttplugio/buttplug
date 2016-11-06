use std::sync::Arc;
use std::sync::mpsc::{Receiver};
use messages;
use messages::{IncomingMessage, Message, Device, Client};
use devices::DeviceInfo;

struct FakeDeviceWrapper {
    connected: bool
}

impl FakeDeviceWrapper {
    pub fn open(path: &str) -> Option<FakeDeviceWrapper> {
        match path {
            "fakedevice1" => {
                Some(FakeDeviceWrapper {
                    connected: true
                })
            },
            _ => { None }
        }
    }

    pub fn handle_message(&mut self, msg: IncomingMessage) {
        if !self.connected {
            (msg.callback)(messages::Error::as_message("Not connected!".to_string()));
        }
        match msg.msg {
            Message::Device(id, dm) => {
                match dm {
                    Device::SingleVibrateSpeed(speed) => {
                        warn!("Got vibrate message!");
                        (msg.callback)(messages::Ok::as_message());
                    },
                    _ => {
                        (msg.callback)(messages::Error::as_message("Cannot parse message!".to_string()));
                    }
                }
            },
            _ => {
                (msg.callback)(messages::Error::as_message("Cannot parse message!".to_string()));
            }
        }
    }

    pub fn close(&mut self) -> Result<(), ()> {
        if !self.connected {
            return Err(());
        }
        self.connected = false;
        Ok(())
    }
}

pub fn discover_devices() -> Vec<DeviceInfo> {
    let mut devices : Vec<DeviceInfo> = Vec::new();
    devices.push(DeviceInfo {
        name: "Fake Device 1".to_string(),
        // TODO This should be a string
        id: 1,
        path: "fakedevice1".to_string(),
        broadcast: false,
        loop_closure: Arc::new(|path: String, recvr: Receiver<IncomingMessage>| {
            let mut device : FakeDeviceWrapper = match FakeDeviceWrapper::open(&path) {
                Some(d) => {
                    warn!("Device opened!");
                    d
                }
                None => {
                    warn!("Device won't open!");
                    //(msg.callback)(messages::Error::as_message("Cannot open device!".to_string()));
                    return;
                }
            };
            loop {
                let msg = recvr.recv().unwrap();
                let parse_msg = msg.msg.clone();
                match parse_msg {
                    Message::Device(id, dm) => {
                        match dm {
                            _ => {
                                warn!("Handling device message!");
                                device.handle_message(msg);
                            }
                        };
                    },
                    _ => {
                        (msg.callback)(messages::Error::as_message("Cannot parse message!".to_string()));
                        break;
                    }
                };
            }
        })
    });
    return devices;
}
