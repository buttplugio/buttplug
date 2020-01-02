use crate::{
    core::{
        errors::{ButtplugDeviceError, ButtplugError},
        messages::{self, ButtplugMessage, RawReadCmd, RawReading, RawWriteCmd},
    },
    devices::{
        configuration_manager::{
            BluetoothLESpecifier, DeviceConfigurationManager, DeviceSpecifier, ProtocolDefinition,
        },
        Endpoint,
    },
    server::device_manager::{
        ButtplugDevice, ButtplugDeviceResponseMessage, ButtplugProtocolRawMessage,
        DeviceCommunicationEvent, DeviceCommunicationManager, DeviceCommunicationManagerCreator,
        DeviceImpl,
    },
    util::future::{ButtplugFuture, ButtplugFutureStateShared},
};
use async_std::{
    prelude::{FutureExt, StreamExt},
    sync::{channel, Receiver, Sender},
    task,
};
use async_trait::async_trait;
use rumble::api::{Central, CentralEvent, Characteristic, Peripheral, UUID};
#[cfg(feature = "linux-ble")]
use rumble::bluez::{adapter::ConnectedAdapter, manager::Manager};
#[cfg(feature = "winrt-ble")]
use rumble::winrtble::{adapter::Adapter, manager::Manager};
use std::{collections::HashMap, thread};
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
        let device_mgr = DeviceConfigurationManager::load_from_internal();
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
            let mut tried_names: Vec<String> = vec![];
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
                            if let Some((protocol_name, protocol)) =
                                device_mgr.find_protocol(&DeviceSpecifier::BluetoothLE(ble_conf))
                            {
                                info!("Found Buttplug Device {}", name);
                                let dev = connect(p, protocol.clone()).await.unwrap();
                                let proto = device_mgr
                                    .create_protocol_impl(
                                        &protocol_name
                                    )
                                    .unwrap();
                                let d = ButtplugDevice::new(proto, Box::new(dev));
                                info!("Sending device connected message!");
                                device_sender
                                    .send(DeviceCommunicationEvent::DeviceAdded(d))
                                    .await;
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

enum ButtplugDeviceCommand {
    Connect,
    Message(ButtplugProtocolRawMessage),
    Disconnect,
}

enum ButtplugDeviceReturn {
    Ok(messages::Ok),
    RawReading(messages::RawReading),
    Error(ButtplugError),
}

type DeviceReturnStateShared = ButtplugFutureStateShared<ButtplugDeviceReturn>;
type DeviceReturnFuture = ButtplugFuture<ButtplugDeviceReturn>;

enum RumbleCommLoopChannelValue {
    DeviceCommand(ButtplugDeviceCommand, DeviceReturnStateShared),
    DeviceOutput(RawReading),
    ChannelClosed,
}

async fn rumble_comm_loop<T: Peripheral>(
    device: T,
    protocol: BluetoothLESpecifier,
    mut write_receiver: Receiver<(ButtplugDeviceCommand, DeviceReturnStateShared)>,
    // TODO implement output sending due to notifications
    mut _output_sender: Sender<ButtplugDeviceResponseMessage>,
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
                    device.connect().unwrap();
                    let chars = device.discover_characteristics().unwrap();
                    for proto_service in protocol.services.values() {
                        info!("Searching services");
                        for (chr_name, chr_uuid) in proto_service.into_iter() {
                            let chr = chars.iter().find(|c| c.uuid == uuid_to_rumble(chr_uuid));
                            if chr.is_some() {
                                info!("Found valid characteristic {}", chr_uuid);
                                endpoints.insert(*chr_name, chr.unwrap().clone());
                            }
                        }
                    }
                    state
                        .lock()
                        .unwrap()
                        .set_reply(ButtplugDeviceReturn::Ok(messages::Ok::new(1)));
                }
                ButtplugDeviceCommand::Message(raw_msg) => match raw_msg {
                    ButtplugProtocolRawMessage::RawWriteCmd(msg) => {
                        match endpoints.get(&msg.endpoint) {
                            Some(chr) => {
                                device.command(&chr, &msg.data).unwrap();
                                state.lock().unwrap().set_reply(ButtplugDeviceReturn::Ok(
                                    messages::Ok::new(msg.get_id()),
                                ));
                            }
                            None => state.lock().unwrap().set_reply(ButtplugDeviceReturn::Error(
                                ButtplugError::ButtplugDeviceError(ButtplugDeviceError::new(
                                    &format!(
                                        "Device does not contain an endpoint named {}",
                                        msg.endpoint
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
            RumbleCommLoopChannelValue::DeviceOutput(_raw_reading) => {},
            RumbleCommLoopChannelValue::ChannelClosed => {}
        }
    }
}

#[derive(Clone)]
pub struct RumbleBLEDeviceImpl {
    thread_sender: Sender<(ButtplugDeviceCommand, DeviceReturnStateShared)>,
}

unsafe impl Send for RumbleBLEDeviceImpl {}
unsafe impl Sync for RumbleBLEDeviceImpl {}

fn uuid_to_rumble(uuid: &uuid::Uuid) -> UUID {
    let mut rumble_uuid = uuid.as_bytes().clone();
    rumble_uuid.reverse();
    UUID::B128(rumble_uuid)
}

pub async fn connect<T: Peripheral + 'static>(
    device: T,
    protocol: ProtocolDefinition,
) -> Result<RumbleBLEDeviceImpl, ButtplugError> {
    if let Some(ref proto) = protocol.btle {
        let (device_sender, device_receiver) = channel(256);
        let (output_sender, _output_receiver) = channel(256);
        let p = proto.clone();
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
            ButtplugDeviceReturn::Ok(_) => Ok(RumbleBLEDeviceImpl {
                thread_sender: device_sender,
            }),
            _ => Err(ButtplugError::ButtplugDeviceError(
                ButtplugDeviceError::new("Cannot connect"),
            )),
        }
    } else {
        panic!("Got a protocol with no Bluetooth Definition!");
    }
}

// TODO Actually fill out device information
#[async_trait]
impl DeviceImpl for RumbleBLEDeviceImpl {
    fn name(&self) -> String {
        //self.device.properties().local_name.unwrap()
        "Whatever".to_owned()
    }

    fn address(&self) -> String {
        //self.device.properties().address.to_string()
        "Whatever".to_owned()
    }
    fn connected(&self) -> bool {
        true
    }
    fn endpoints(&self) -> Vec<Endpoint> {
        //self.endpoints.keys().map(|v| v.clone()).collect::<Vec<Endpoint>>()
        vec!()
    }
    fn disconnect(&self) {
        todo!("implement disconnect");
    }
    fn box_clone(&self) -> Box<dyn DeviceImpl> {
        Box::new((*self).clone())
    }

    async fn write_value(&self, msg: &RawWriteCmd) -> Result<(), ButtplugError> {
        let fut = DeviceReturnFuture::default();
        let waker = fut.get_state_clone();
        self.thread_sender
            .send((
                ButtplugDeviceCommand::Message(ButtplugProtocolRawMessage::RawWriteCmd(
                    msg.clone(),
                )),
                waker,
            ))
            .await;
        match fut.await {
            ButtplugDeviceReturn::Ok(_) => Ok(()),
            _ => Err(ButtplugError::ButtplugDeviceError(
                ButtplugDeviceError::new("Cannot connect"),
            )),
        }
    }

    async fn read_value(&self, msg: &RawReadCmd) -> Result<RawReading, ButtplugError> {
        // TODO Actually implement value reading
        Ok(RawReading::new(0, msg.endpoint, vec![]))
    }
}

#[cfg(all(test, any(feature="winrt-ble", feature="linux-ble")))]
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
    pub fn test_rumble() {
        let _ = env_logger::builder().is_test(true).try_init();
        task::block_on(async move {
            let (sender, mut receiver) = channel(256);
            let mut mgr = RumbleBLECommunicationManager::new(sender);
            mgr.start_scanning().await;
            loop {
                match receiver.next().await.unwrap() {
                    DeviceCommunicationEvent::DeviceAdded(mut device) => {
                        info!("Got device!");
                        info!("Sending message!");
                        match device
                            .parse_message(
                                &VibrateCmd::new(1, vec![VibrateSubcommand::new(0, 0.5)]).into(),
                            )
                            .await
                        {
                            Ok(msg) => match msg {
                                ButtplugMessageUnion::Ok(_) => info!("Returned Ok"),
                                _ => info!("Returned something other than ok"),
                            },
                            Err(_) => {
                                assert!(false, "Error returned from parse message");
                            }
                        }
                    }
                    _ => assert!(false, "Shouldn't get other message types!"),
                }
            }
        });
    }
}
