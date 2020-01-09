use crate::{
    core::{
        errors::{ButtplugDeviceError, ButtplugError},
        messages::{self, RawReading},
    },
    device::{
        device::{
            ButtplugDeviceCommand, ButtplugDeviceEvent,
            ButtplugDeviceImplInfo, ButtplugDeviceReturn, DeviceImplCommand,
            DeviceReadCmd, DeviceSubscribeCmd, DeviceUnsubscribeCmd, DeviceWriteCmd,
        },
        configuration_manager::BluetoothLESpecifier,
        Endpoint,
    },
    util::future::{ButtplugFuture, ButtplugFutureStateShared},
};
use async_std::{
    prelude::{FutureExt, StreamExt},
    sync::{channel, Receiver, Sender},
    task,
};
use rumble::api::{Central, CentralEvent, Characteristic, Peripheral, ValueNotification, UUID};
use std::collections::HashMap;
use uuid;

pub type DeviceReturnStateShared = ButtplugFutureStateShared<ButtplugDeviceReturn>;
pub type DeviceReturnFuture = ButtplugFuture<ButtplugDeviceReturn>;

enum RumbleCommLoopChannelValue {
    DeviceCommand(ButtplugDeviceCommand, DeviceReturnStateShared),
    DeviceEvent(CentralEvent),
    ChannelClosed,
}

pub struct RumbleInternalEventLoop<T: Peripheral> {
    device: T,
    protocol: BluetoothLESpecifier,
    write_receiver: Receiver<(ButtplugDeviceCommand, DeviceReturnStateShared)>,
    event_receiver: Receiver<CentralEvent>,
    output_sender: Sender<ButtplugDeviceEvent>,
    endpoints: HashMap<Endpoint, Characteristic>,
}

fn uuid_to_rumble(uuid: &uuid::Uuid) -> UUID {
    let mut rumble_uuid = uuid.as_bytes().clone();
    rumble_uuid.reverse();
    UUID::B128(rumble_uuid)
}

impl<T: Peripheral> RumbleInternalEventLoop<T> {
    pub fn new<C>(central: C,
                  device: T,
                  protocol: BluetoothLESpecifier,
                  write_receiver: Receiver<(ButtplugDeviceCommand, DeviceReturnStateShared)>,
                  output_sender: Sender<ButtplugDeviceEvent>) -> Self
    where C: Central<T> {
        let (event_sender, event_receiver) = channel(256);
        // Add ourselves to the central event handler output now, so we don't
        // have to carry around the Central object. We'll be using this in
        // connect anyways.
        let on_event = move |event: CentralEvent| match event {
            CentralEvent::DeviceConnected(_) => {
                let s = event_sender.clone();
                let e = event.clone();
                task::spawn(async move {
                    s.send(e).await;
                });
            },
            CentralEvent::DeviceDisconnected(_) => {
                let s = event_sender.clone();
                let e = event.clone();
                task::spawn(async move {
                    s.send(e).await;
                });
            },
            _ => {}
        };
        // TODO There's no way to unsubscribe central event handlers. That
        // needs to be fixed in rumble somehow, but for now we'll have to
        // make our handlers exit early after dying or something?
        central.on_event(Box::new(on_event));
        RumbleInternalEventLoop {
            device,
            protocol,
            write_receiver,
            event_receiver,
            output_sender,
            endpoints: HashMap::new()
        }
    }

    async fn handle_connection(&mut self, state: &mut DeviceReturnStateShared) {
        info!("Connecting to device!");
        self.device.connect().unwrap();
        loop {
            match self.event_receiver.next().await.unwrap() {
                CentralEvent::DeviceConnected(addr) => {
                    if addr == self.device.address() {
                        info!("Device {:?} connected!", self.device.properties().local_name);
                        break;
                    }
                }
                _ => {}
            }
        }
        // Rumble only gives you the u16 endpoint handle during
        // notifications so we've gotta create yet another mapping.
        let mut handle_map = HashMap::<u16, Endpoint>::new();
        let chars = self.device.discover_characteristics().unwrap();
        for proto_service in self.protocol.services.values() {
            for (chr_name, chr_uuid) in proto_service.into_iter() {
                let maybe_chr =
                    chars.iter().find(|c| c.uuid == uuid_to_rumble(chr_uuid));
                if let Some(chr) = maybe_chr {
                    self.endpoints.insert(*chr_name, chr.clone());
                    handle_map.insert(chr.value_handle, *chr_name);
                }
            }
        }
        let os = self.output_sender.clone();
        self.device.on_notification(Box::new(move |notification: ValueNotification| {
            let endpoint = handle_map.get(&notification.handle).unwrap().clone();
            let sender = os.clone();
            task::spawn(async move {
                sender
                    .send(ButtplugDeviceEvent::Notification(
                        endpoint,
                        notification.value,
                    ))
                    .await
            });
        }));
        let device_info = ButtplugDeviceImplInfo {
            endpoints: self.endpoints.keys().cloned().collect(),
            manufacturer_name: None,
            product_name: None,
            serial_number: None,
        };
        info!("Device connected!");
        state
            .lock()
            .unwrap()
            .set_reply(ButtplugDeviceReturn::Connected(device_info));
    }

    fn handle_write(&mut self, write_msg: &DeviceWriteCmd, state: &mut DeviceReturnStateShared) {
        match self.endpoints.get(&write_msg.endpoint) {
            Some(chr) => {
                self.device.command(&chr, &write_msg.data).unwrap();
                state
                    .lock()
                    .unwrap()
                    .set_reply(ButtplugDeviceReturn::Ok(messages::Ok::default()));
            }
            None => state.lock().unwrap().set_reply(ButtplugDeviceReturn::Error(
                ButtplugError::ButtplugDeviceError(ButtplugDeviceError::new(
                    &format!(
                        "Device does not contain an endpoint named {}",
                        write_msg.endpoint
                    )
                        .to_owned(),
                )),
            )),
        }
    }

    fn handle_subscribe(&mut self, sub_msg: &DeviceSubscribeCmd, state: &mut DeviceReturnStateShared) {
        match self.endpoints.get(&sub_msg.endpoint) {
            Some(chr) => {
                self.device.subscribe(&chr).unwrap();
                state
                    .lock()
                    .unwrap()
                    .set_reply(ButtplugDeviceReturn::Ok(messages::Ok::default()));
            }
            None => state.lock().unwrap().set_reply(ButtplugDeviceReturn::Error(
                ButtplugError::ButtplugDeviceError(ButtplugDeviceError::new(
                    &format!(
                        "Device does not contain an endpoint named {}",
                        sub_msg.endpoint
                    )
                        .to_owned(),
                )),
            )),
        }
    }

    fn handle_unsubscribe(&mut self, sub_msg: &DeviceUnsubscribeCmd, state: &mut DeviceReturnStateShared) {
        match self.endpoints.get(&sub_msg.endpoint) {
            Some(chr) => {
                self.device.subscribe(&chr).unwrap();
                state
                    .lock()
                    .unwrap()
                    .set_reply(ButtplugDeviceReturn::Ok(messages::Ok::default()));
            }
            None => state.lock().unwrap().set_reply(ButtplugDeviceReturn::Error(
                ButtplugError::ButtplugDeviceError(ButtplugDeviceError::new(
                    &format!(
                        "Device does not contain an endpoint named {}",
                        sub_msg.endpoint
                    )
                        .to_owned(),
                )),
            )),
        }
    }

    pub async fn handle_device_command(&mut self, command: &ButtplugDeviceCommand, state: &mut DeviceReturnStateShared) {
        match command {
            ButtplugDeviceCommand::Connect => {
                self.handle_connection(state).await;
            }
            ButtplugDeviceCommand::Message(raw_msg) => match raw_msg {
                DeviceImplCommand::Write(write_msg) => {
                    self.handle_write(write_msg, state);
                }
                DeviceImplCommand::Subscribe(sub_msg) => {
                    self.handle_subscribe(sub_msg, state);
                }
                DeviceImplCommand::Unsubscribe(sub_msg) => {
                    self.handle_unsubscribe(sub_msg, state);
                }
                _ => state.lock().unwrap().set_reply(ButtplugDeviceReturn::Error(
                    ButtplugError::ButtplugDeviceError(ButtplugDeviceError::new(
                        "Buttplug-rs does not yet handle reads",
                    )),
                )),
            },
            ButtplugDeviceCommand::Disconnect => {
                self.device.disconnect();
            }
        }
    }

    pub async fn handle_device_event(&mut self, event: &CentralEvent) {
        match event {
            // TODO Ok. Great. We can disconnect, but output_sender doesn't
            // really *go* anywhere right now. We're just using it in the
            // Lovense protocol and that's it. We need to be watching for this
            // up in the device manager too, which is going to be...
            // interesting, as I have no idea how we'll deal with instances
            // where we disconnect while waiting in a protocol (for instance, if
            // the device disconnects while we're doing Lovense init). I may
            // need to rethink this.
            CentralEvent::DeviceDisconnected(addr) => {
                if self.device.address() == *addr {
                    info!("Device {:?} disconnected", self.device.properties().local_name);
                    self.output_sender.send(ButtplugDeviceEvent::Removed).await;
                }
            },
            _ => {}
        }
    }

    pub async fn run(&mut self) {
        loop {
            let mut wr = self.write_receiver.clone();
            let receiver = async {
                match wr.next().await {
                    Some((command, state)) => RumbleCommLoopChannelValue::DeviceCommand(command, state),
                    None => RumbleCommLoopChannelValue::ChannelClosed,
                }
            };
            let mut er = self.event_receiver.clone();
            let event = async {
                // We own both sides of this so it'll never actually die. Unwrap
                // with impunity.
                RumbleCommLoopChannelValue::DeviceEvent(er.next().await.unwrap())
            };
            // Race our device input (from the client side) and any subscribed
            // notifications.
            match receiver.race(event).await {
                RumbleCommLoopChannelValue::DeviceCommand(ref command, ref mut state) => self.handle_device_command(command, state).await,
                RumbleCommLoopChannelValue::DeviceEvent(event) => self.handle_device_event(&event).await,
                RumbleCommLoopChannelValue::ChannelClosed => {}
            }
        }
    }
}
