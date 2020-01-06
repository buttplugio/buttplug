use crate::{
    core::{
        errors::ButtplugError,
        messages::{
            self, ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion, RawReadCmd, RawReading,
            RawWriteCmd, SubscribeCmd, UnsubscribeCmd,
        },
    },
    device::{
        protocol::ButtplugProtocol,
        Endpoint,
        configuration_manager::{DeviceSpecifier, ProtocolDefinition, DeviceConfigurationManager},
    },
};
use async_std::sync::Receiver;
use async_trait::async_trait;

pub struct DeviceReadCmd {
    pub endpoint: Endpoint,
    pub length: u32,
    pub timeout_ms: u32,
}

impl DeviceReadCmd {
    pub fn new(endpoint: Endpoint, length: u32, timeout_ms: u32) -> Self {
        Self {
            endpoint,
            length,
            timeout_ms,
        }
    }
}

impl From<RawReadCmd> for DeviceReadCmd {
    fn from(msg: RawReadCmd) -> Self {
        Self {
            endpoint: msg.endpoint,
            length: msg.expected_length,
            timeout_ms: msg.timeout,
        }
    }
}

pub struct DeviceWriteCmd {
    pub endpoint: Endpoint,
    pub data: Vec<u8>,
    pub write_with_response: bool,
}

impl DeviceWriteCmd {
    pub fn new(endpoint: Endpoint, data: Vec<u8>, write_with_response: bool) -> Self {
        Self {
            endpoint,
            data,
            write_with_response,
        }
    }
}

impl From<RawWriteCmd> for DeviceWriteCmd {
    fn from(msg: RawWriteCmd) -> Self {
        Self {
            endpoint: msg.endpoint,
            data: msg.data,
            write_with_response: msg.write_with_response,
        }
    }
}

pub struct DeviceSubscribeCmd {
    pub endpoint: Endpoint,
}

impl DeviceSubscribeCmd {
    pub fn new(endpoint: Endpoint) -> Self {
        Self { endpoint }
    }
}

impl From<SubscribeCmd> for DeviceSubscribeCmd {
    fn from(msg: SubscribeCmd) -> Self {
        Self {
            endpoint: msg.endpoint,
        }
    }
}

pub struct DeviceUnsubscribeCmd {
    pub endpoint: Endpoint,
}

impl DeviceUnsubscribeCmd {
    pub fn new(endpoint: Endpoint) -> Self {
        Self { endpoint }
    }
}

impl From<UnsubscribeCmd> for DeviceUnsubscribeCmd {
    fn from(msg: UnsubscribeCmd) -> Self {
        Self {
            endpoint: msg.endpoint,
        }
    }
}

pub enum DeviceImplCommand {
    // Endpoint, data, write with response
    Write(DeviceWriteCmd),
    // Endpoint, length, timeout in ms
    Read(DeviceReadCmd),
    Subscribe(DeviceSubscribeCmd),
    Unsubscribe(DeviceUnsubscribeCmd),
}

impl From<RawReadCmd> for DeviceImplCommand {
    fn from(msg: RawReadCmd) -> Self {
        DeviceImplCommand::Read(msg.into())
    }
}

impl From<RawWriteCmd> for DeviceImplCommand {
    fn from(msg: RawWriteCmd) -> Self {
        DeviceImplCommand::Write(msg.into())
    }
}

impl From<SubscribeCmd> for DeviceImplCommand {
    fn from(msg: SubscribeCmd) -> Self {
        DeviceImplCommand::Subscribe(msg.into())
    }
}

impl From<UnsubscribeCmd> for DeviceImplCommand {
    fn from(msg: UnsubscribeCmd) -> Self {
        DeviceImplCommand::Unsubscribe(msg.into())
    }
}

impl From<DeviceReadCmd> for DeviceImplCommand {
    fn from(msg: DeviceReadCmd) -> Self {
        DeviceImplCommand::Read(msg)
    }
}

impl From<DeviceWriteCmd> for DeviceImplCommand {
    fn from(msg: DeviceWriteCmd) -> Self {
        DeviceImplCommand::Write(msg)
    }
}

impl From<DeviceSubscribeCmd> for DeviceImplCommand {
    fn from(msg: DeviceSubscribeCmd) -> Self {
        DeviceImplCommand::Subscribe(msg)
    }
}

impl From<DeviceUnsubscribeCmd> for DeviceImplCommand {
    fn from(msg: DeviceUnsubscribeCmd) -> Self {
        DeviceImplCommand::Unsubscribe(msg)
    }
}

pub struct ButtplugDeviceImplInfo {
    pub endpoints: Vec<Endpoint>,
    pub manufacturer_name: Option<String>,
    pub product_name: Option<String>,
    pub serial_number: Option<String>,
}

pub enum ButtplugDeviceCommand {
    Connect,
    Message(DeviceImplCommand),
    Disconnect,
}

pub enum ButtplugDeviceReturn {
    Connected(ButtplugDeviceImplInfo),
    Ok(messages::Ok),
    RawReading(messages::RawReading),
    Error(ButtplugError),
}

#[derive(Debug)]
pub enum ButtplugDeviceEvent {
    Notification(Endpoint, Vec<u8>),
    Removed,
}

#[async_trait]
pub trait DeviceImpl: Sync + Send {
    fn name(&self) -> &str;
    fn address(&self) -> &str;
    fn connected(&self) -> bool;
    fn endpoints(&self) -> Vec<Endpoint>;
    fn disconnect(&self);
    fn box_clone(&self) -> Box<dyn DeviceImpl>;
    fn get_event_receiver(&self) -> Receiver<ButtplugDeviceEvent>;

    // TODO Taking messages mean we have to form full messages in the protocol.
    // This seems silly. We can probably make stripped down versions to send
    // that don't have message IDs or device indexes.
    async fn read_value(&self, msg: DeviceReadCmd) -> Result<RawReading, ButtplugError>;
    async fn write_value(&self, msg: DeviceWriteCmd) -> Result<(), ButtplugError>;
    async fn subscribe(&self, msg: DeviceSubscribeCmd) -> Result<(), ButtplugError>;
    async fn unsubscribe(&self, msg: DeviceUnsubscribeCmd) -> Result<(), ButtplugError>;
}

impl Clone for Box<dyn DeviceImpl> {
    fn clone(&self) -> Box<dyn DeviceImpl> {
        self.box_clone()
    }
}

#[async_trait]
pub trait ButtplugDeviceImplCreator: Sync + Send {
    fn get_specifier(&self) -> DeviceSpecifier;
    async fn try_create_device_impl(&mut self, protocol: ProtocolDefinition) -> Result<Box<dyn DeviceImpl>, ButtplugError>;
}

pub struct ButtplugDevice {
    protocol: Box<dyn ButtplugProtocol>,
    device: Box<dyn DeviceImpl>,
}

impl Clone for ButtplugDevice {
    fn clone(&self) -> Self {
        ButtplugDevice {
            protocol: self.protocol.clone(),
            device: self.device.clone(),
        }
    }
}

impl ButtplugDevice {
    pub fn new(protocol: Box<dyn ButtplugProtocol>, device: Box<dyn DeviceImpl>) -> Self {
        Self { protocol, device }
    }

    pub async fn try_create_device(mut device_creator: Box<dyn ButtplugDeviceImplCreator>) -> Result<Option<ButtplugDevice>, ButtplugError> {
        let device_mgr = DeviceConfigurationManager::new();
        // First off, we need to see if we even have a configuration available
        // for the device we're trying to create. If we don't, return Ok(None),
        // because this isn't actually an error. However, if we *do* have a
        // configuration but something goes wrong after this, then it's an
        // error.

        match device_mgr.find_configuration(&device_creator.get_specifier()) {
            Some((config_name, config)) => {
                // Now that we have both a possible device implementation and a
                // configuration for that device, try to initialize the implementation.
                // This usually means trying to connect to whatever the device is,
                // finding endpoints, etc.
                match device_creator.try_create_device_impl(config).await {
                    Ok(device_impl) => {
                        info!("Found Buttplug Device {}", device_impl.name());
                        // If we've made it this far, we now have a connected device
                        // implementation with endpoints set up. We now need to run whatever
                        // protocol initialization might need to happen. We'll fetch a protocol
                        // creator, pass the device implementation to it, then let it do
                        // whatever it needs. For most protocols, this is a no-op. However, for
                        // devices like Lovense, some Kiiroo, etc, this can get fairly
                        // complicated.

                        let proto_creator =
                            device_mgr.get_protocol_creator(&config_name).unwrap();
                        match proto_creator.try_create_protocol(&device_impl).await {
                            Ok(protocol_impl) => {
                                Ok(Some(ButtplugDevice::new(protocol_impl, device_impl)))
                            },
                            Err(e) => Err(e)
                        }
                    },
                    Err(e) => Err(e)
                }
            },
            None => return Ok(None)
        }

    }

    pub fn name(&self) -> &str {
        self.device.name()
    }

    pub async fn parse_message(
        &mut self,
        message: &ButtplugDeviceCommandMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        self.protocol.parse_message(&self.device, message).await
    }
}
