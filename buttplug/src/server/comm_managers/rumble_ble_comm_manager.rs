use crate::{
    core::{
        errors::{ButtplugDeviceError, ButtplugError},
        messages::{self, RawReading},
    },
    device::{
        configuration_manager::{
            BluetoothLESpecifier, DeviceSpecifier, ProtocolDefinition
        },
        device::{
            ButtplugDeviceEvent, DeviceImpl, DeviceImplCommand, DeviceReadCmd,
            DeviceSubscribeCmd, DeviceUnsubscribeCmd, DeviceWriteCmd, ButtplugDeviceImplCreator,
            ButtplugDeviceImplInfo, ButtplugDeviceReturn, ButtplugDeviceCommand
        },
        Endpoint,
    },
    server::device_manager::{
        DeviceCommunicationEvent, DeviceCommunicationManager, DeviceCommunicationManagerCreator,
    },
    util::future::{ButtplugFuture, ButtplugFutureStateShared},
};
use async_std::{
    prelude::{FutureExt, StreamExt},
    sync::{channel, Receiver, Sender},
    task,
};
use async_trait::async_trait;
use rumble::api::{Central, CentralEvent, Characteristic, Peripheral, ValueNotification, UUID};
#[cfg(feature = "linux-ble")]
use rumble::bluez::{adapter::ConnectedAdapter, manager::Manager};
#[cfg(feature = "winrt-ble")]
use rumble::winrtble::{adapter::Adapter, manager::Manager};
use std::collections::HashMap;
use uuid;

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

type DeviceReturnStateShared = ButtplugFutureStateShared<ButtplugDeviceReturn>;
type DeviceReturnFuture = ButtplugFuture<ButtplugDeviceReturn>;

enum RumbleCommLoopChannelValue {
    DeviceCommand(ButtplugDeviceCommand, DeviceReturnStateShared),
    DeviceOutput(RawReading),
    ChannelClosed,
}

// TODO There is way, way too much shit happening in here. Break it down into
// smaller bits, possibly as a struct.
async fn rumble_comm_loop<T: Peripheral>(
    device: T,
    protocol: BluetoothLESpecifier,
    mut write_receiver: Receiver<(ButtplugDeviceCommand, DeviceReturnStateShared)>,
    output_sender: Sender<ButtplugDeviceEvent>,
) {
    // TODO How the do we deal with disconnection, as well as spinning down the
    // thread during shutdown?

    // We'll handle all notifications from a device on a single channel, because
    // there's no way bluetooth is going to flood us with enough data to
    // saturate, right? (I am prepared to regret this.)
    //
    // Any time we get a request to subscribe somewhere, just load the callback
    // with the same sender everyone is using and treat this as a mpsc.
    let (_notification_sender, mut notification_receiver) = channel::<RawReading>(256);
    let mut endpoints = HashMap::<Endpoint, Characteristic>::new();
    loop {
        let receiver = async {
            match write_receiver.next().await {
                Some((command, state)) => RumbleCommLoopChannelValue::DeviceCommand(command, state),
                None => RumbleCommLoopChannelValue::ChannelClosed,
            }
        };
        let notification = async {
            // We own both sides of this so it'll never actually die. Unwrap
            // with impunity.
            RumbleCommLoopChannelValue::DeviceOutput(notification_receiver.next().await.unwrap())
        };
        // Race our device input (from the client side) and any subscribed
        // notifications.
        match receiver.race(notification).await {
            RumbleCommLoopChannelValue::DeviceCommand(command, state) => match command {
                ButtplugDeviceCommand::Connect => {
                    info!("Connecting to device!");
                    device.connect().unwrap();
                    // Rumble only gives you the u16 endpoint handle during
                    // notifications so we've gotta create yet another mapping.
                    let mut handle_map = HashMap::<u16, Endpoint>::new();
                    let chars = device.discover_characteristics().unwrap();
                    for proto_service in protocol.services.values() {
                        for (chr_name, chr_uuid) in proto_service.into_iter() {
                            let maybe_chr =
                                chars.iter().find(|c| c.uuid == uuid_to_rumble(chr_uuid));
                            if let Some(chr) = maybe_chr {
                                endpoints.insert(*chr_name, chr.clone());
                                handle_map.insert(chr.value_handle, *chr_name);
                            }
                        }
                    }
                    let os = output_sender.clone();
                    device.on_notification(Box::new(move |notification: ValueNotification| {
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
                        endpoints: endpoints.keys().cloned().collect(),
                        manufacturer_name: None,
                        product_name: None,
                        serial_number: None
                    };
                    info!("Device connected!");
                    state
                        .lock()
                        .unwrap()
                        .set_reply(ButtplugDeviceReturn::Connected(device_info));
                }
                ButtplugDeviceCommand::Message(raw_msg) => match raw_msg {
                    DeviceImplCommand::Write(write_msg) => {
                        match endpoints.get(&write_msg.endpoint) {
                            Some(chr) => {
                                device.command(&chr, &write_msg.data).unwrap();
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
                    DeviceImplCommand::Subscribe(sub_msg) => {
                        match endpoints.get(&sub_msg.endpoint) {
                            Some(chr) => {
                                device.subscribe(&chr).unwrap();
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
                    DeviceImplCommand::Unsubscribe(sub_msg) => {
                        match endpoints.get(&sub_msg.endpoint) {
                            Some(chr) => {
                                device.unsubscribe(&chr).unwrap();
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
                    _ => state.lock().unwrap().set_reply(ButtplugDeviceReturn::Error(
                        ButtplugError::ButtplugDeviceError(ButtplugDeviceError::new(
                            "Buttplug-rs does not yet handle reads",
                        )),
                    )),
                },
                ButtplugDeviceCommand::Disconnect => {}
            },
            // TODO implement output sending
            RumbleCommLoopChannelValue::DeviceOutput(_raw_reading) => {}
            RumbleCommLoopChannelValue::ChannelClosed => {}
        }
    }
}

pub struct RumbleBLEDeviceImplCreator<T: Peripheral + 'static> {
    device: Option<T>
}

impl<T: Peripheral> RumbleBLEDeviceImplCreator<T> {
    pub fn new(device: T) -> Self {
        Self {
            device: Some(device)
        }
    }
}

#[async_trait]
impl<T: Peripheral> ButtplugDeviceImplCreator for RumbleBLEDeviceImplCreator<T> {
    fn get_specifier(&self) -> DeviceSpecifier {
        if self.device.is_none() {
            panic!("Cannot call get_specifier after device is taken!");
        }
        let name = self.device.as_ref().unwrap().properties().local_name.unwrap();
        DeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(&name))
    }

    async fn try_create_device_impl(&mut self, protocol: ProtocolDefinition)
                                    -> Result<Box<dyn DeviceImpl>, ButtplugError> {
        // TODO ugggggggh there's gotta be a way to ensure this at compile time.
        if self.device.is_none() {
            panic!("Cannot call try_create_device_impl twice!");
        }
        let device = self.device.take().unwrap();
        if let Some(ref proto) = protocol.btle {
            let (device_sender, device_receiver) = channel(256);
            let (output_sender, output_receiver) = channel(256);
            let p = proto.clone();
            let name = device.properties().local_name.unwrap();
            let address = device.properties().address.to_string();
            // TODO This is not actually async. We're currently using blocking
            // rumble calls, so this will block whatever thread it's spawned to. We
            // should probably switch to using async rumble calls w/ callbacks.
            //
            // The new watchdog async-std executor will at least leave this task on
            // its own thread in time, but I'm not sure when that's landing.
            task::spawn(async move {
                rumble_comm_loop(device, p, device_receiver, output_sender).await;
            });
            let fut = DeviceReturnFuture::default();
            let waker = fut.get_state_clone();
            device_sender
                .send((ButtplugDeviceCommand::Connect, waker))
                .await;
            match fut.await {
                ButtplugDeviceReturn::Connected(info) => Ok(Box::new(RumbleBLEDeviceImpl {
                    name,
                    address,
                    endpoints: info.endpoints,
                    thread_sender: device_sender,
                    event_receiver: output_receiver,
                })),
                _ => Err(ButtplugError::ButtplugDeviceError(
                    ButtplugDeviceError::new("Cannot connect"),
                )),
            }
        } else {
            panic!("Got a protocol with no Bluetooth Definition!");
        }
    }
}

#[derive(Clone)]
pub struct RumbleBLEDeviceImpl {
    name: String,
    address: String,
    endpoints: Vec<Endpoint>,
    thread_sender: Sender<(ButtplugDeviceCommand, DeviceReturnStateShared)>,
    event_receiver: Receiver<ButtplugDeviceEvent>,
}

unsafe impl Send for RumbleBLEDeviceImpl {}
unsafe impl Sync for RumbleBLEDeviceImpl {}

fn uuid_to_rumble(uuid: &uuid::Uuid) -> UUID {
    let mut rumble_uuid = uuid.as_bytes().clone();
    rumble_uuid.reverse();
    UUID::B128(rumble_uuid)
}

impl RumbleBLEDeviceImpl {
    pub fn new(name: &String,
               address: &String,
               endpoints: Vec<Endpoint>,
               thread_sender: Sender<(ButtplugDeviceCommand, DeviceReturnStateShared)>,
               event_receiver: Receiver<ButtplugDeviceEvent>) -> Self {
        Self {
            name: name.to_string(),
            address: address.to_string(),
            endpoints,
            thread_sender,
            event_receiver,
        }
    }

    async fn send_to_device_task(
        &self,
        cmd: ButtplugDeviceCommand,
        err_msg: &str,
    ) -> Result<(), ButtplugError> {
        let fut = DeviceReturnFuture::default();
        let waker = fut.get_state_clone();
        self.thread_sender.send((cmd, waker)).await;
        match fut.await {
            ButtplugDeviceReturn::Ok(_) => Ok(()),
            _ => Err(ButtplugError::ButtplugDeviceError(
                ButtplugDeviceError::new(err_msg),
            )),
        }
    }
}

#[async_trait]
impl DeviceImpl for RumbleBLEDeviceImpl {
    fn get_event_receiver(&self) -> Receiver<ButtplugDeviceEvent> {
        self.event_receiver.clone()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn address(&self) -> &str {
        &self.address
    }

    fn connected(&self) -> bool {
        // TODO Should figure out how we wanna deal with this across the
        // representation and inner loop.
        true
    }

    fn endpoints(&self) -> Vec<Endpoint> {
        self.endpoints.clone()
    }
    fn disconnect(&self) {
        todo!("implement disconnect");
    }
    fn box_clone(&self) -> Box<dyn DeviceImpl> {
        Box::new((*self).clone())
    }

    async fn write_value(&self, msg: DeviceWriteCmd) -> Result<(), ButtplugError> {
        self.send_to_device_task(
            ButtplugDeviceCommand::Message(msg.into()),
            "Cannot write to endpoint",
        )
        .await
    }

    async fn read_value(&self, msg: DeviceReadCmd) -> Result<RawReading, ButtplugError> {
        // TODO Actually implement value reading
        Ok(RawReading::new(0, msg.endpoint, vec![]))
    }

    async fn subscribe(&self, msg: DeviceSubscribeCmd) -> Result<(), ButtplugError> {
        self.send_to_device_task(
            ButtplugDeviceCommand::Message(msg.into()),
            "Cannot subscribe",
        )
        .await
    }

    async fn unsubscribe(&self, msg: DeviceUnsubscribeCmd) -> Result<(), ButtplugError> {
        self.send_to_device_task(
            ButtplugDeviceCommand::Message(msg.into()),
            "Cannot unsubscribe",
        )
        .await
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
