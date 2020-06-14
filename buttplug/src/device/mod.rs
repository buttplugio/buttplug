pub mod configuration_manager;
pub mod protocol;
#[cfg(feature = "serialize_json")]
use serde::{
  de::{self, Visitor},
  Deserialize,
  Deserializer,
  Serialize,
  Serializer,
};
use std::{fmt, str::FromStr, string::ToString, convert::TryFrom};

use crate::{
  core::{
    ButtplugResultFuture,
    errors::ButtplugError,
    messages::{
      self,
      ButtplugDeviceCommandMessageUnion,
      MessageAttributesMap,
      RawReadCmd,
      RawReading,
      RawWriteCmd,
      SubscribeCmd,
      UnsubscribeCmd,
    },
  },
  device::{
    configuration_manager::{DeviceConfigurationManager, DeviceSpecifier, ProtocolDefinition},
    protocol::{ButtplugProtocol, ProtocolTypes}
  },
  server::ButtplugServerResultFuture,
};
use async_trait::async_trait;
use broadcaster::BroadcastChannel;
use futures::future::BoxFuture;
use core::hash::{Hash, Hasher};
use configuration_manager::DeviceProtocolConfiguration;

#[derive(EnumString, Clone, Debug, PartialEq, Eq, Hash, Display, Copy)]
#[strum(serialize_all = "lowercase")]
pub enum Endpoint {
  Tx,
  Rx,
  RxPressure,
  RxTouch,
  RxAccel,
  Command,
  Firmware,
  TxMode,
  TxVibrate,
  TxShock,
  TxVendorControl,
  Whitelist,
}

impl Serialize for Endpoint {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(&self.to_string())
  }
}

struct EndpointVisitor;

impl<'de> Visitor<'de> for EndpointVisitor {
  type Value = Endpoint;

  fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    formatter.write_str("a string representing an endpoint")
  }

  fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
  where
    E: de::Error,
  {
    Endpoint::from_str(value).map_err(|e| E::custom(format!("{}", e)))
  }
}

impl<'de> Deserialize<'de> for Endpoint {
  fn deserialize<D>(deserializer: D) -> Result<Endpoint, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer.deserialize_str(EndpointVisitor)
  }
}

pub type BoundedDeviceEventBroadcaster = BroadcastChannel<
  ButtplugDeviceEvent,
  futures_channel::mpsc::Sender<ButtplugDeviceEvent>,
  futures_channel::mpsc::Receiver<ButtplugDeviceEvent>,
>;

#[derive(PartialEq, Debug)]
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

#[derive(PartialEq, Debug)]
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

#[derive(PartialEq, Debug)]
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

#[derive(PartialEq, Debug)]
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

#[derive(PartialEq, Debug)]
pub enum DeviceImplCommand {
  // Endpoint, data, write with response
  Write(DeviceWriteCmd),
  // Endpoint, length, timeout in ms
  Read(DeviceReadCmd),
  Subscribe(DeviceSubscribeCmd),
  Unsubscribe(DeviceUnsubscribeCmd),
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

#[derive(Debug, Clone)]
pub enum ButtplugDeviceEvent {
  Notification(Endpoint, Vec<u8>),
  Removed,
}

pub trait DeviceImpl: Sync + Send {
  fn name(&self) -> &str;
  fn address(&self) -> &str;
  fn connected(&self) -> bool;
  fn endpoints(&self) -> Vec<Endpoint>;
  fn get_event_receiver(&self) -> BoundedDeviceEventBroadcaster;

  fn disconnect(&self) -> ButtplugResultFuture;
  fn read_value(&self, msg: DeviceReadCmd) -> BoxFuture<'static, Result<RawReading, ButtplugError>>;
  fn write_value(&self, msg: DeviceWriteCmd) -> ButtplugResultFuture;
  fn subscribe(&self, msg: DeviceSubscribeCmd) -> ButtplugResultFuture;
  fn unsubscribe(&self, msg: DeviceUnsubscribeCmd) -> ButtplugResultFuture;
}

#[async_trait]
pub trait ButtplugDeviceImplCreator: Sync + Send {
  fn get_specifier(&self) -> DeviceSpecifier;
  async fn try_create_device_impl(
    &mut self,
    protocol: ProtocolDefinition,
  ) -> Result<Box<dyn DeviceImpl>, ButtplugError>;
}

#[derive(ShallowCopy)]
pub struct ButtplugDevice {
  protocol: Box<dyn ButtplugProtocol>,
  device: Box<dyn DeviceImpl>,
}

impl Hash for ButtplugDevice {
  fn hash<H: Hasher>(&self, state: &mut H) {
      self.device.address().hash(state);
  }
}

impl Eq for ButtplugDevice {}

impl PartialEq for ButtplugDevice {
  fn eq(&self, other: &Self) -> bool {
    self.device.address() == other.device.address()
  }
}

impl ButtplugDevice {
  pub fn new(protocol: Box<dyn ButtplugProtocol>, device: Box<dyn DeviceImpl>) -> Self {
    Self { protocol, device }
  }

  pub async fn try_create_device(
    mut device_creator: Box<dyn ButtplugDeviceImplCreator>,
  ) -> Result<Option<ButtplugDevice>, ButtplugError> {
    let device_cfg_mgr = DeviceConfigurationManager::default();
    // First off, we need to see if we even have a configuration available
    // for the device we're trying to create. If we don't, return Ok(None),
    // because this isn't actually an error. However, if we *do* have a
    // configuration but something goes wrong after this, then it's an
    // error.

    match device_cfg_mgr.find_configuration(&device_creator.get_specifier()) {
      Some((config_name, config)) => {
        // Now that we have both a possible device implementation and a
        // configuration for that device, try to initialize the implementation.
        // This usually means trying to connect to whatever the device is,
        // finding endpoints, etc.
        let device_protocol_config = DeviceProtocolConfiguration::new(
          config.defaults.clone(),
          config.configurations.clone(),
        );
        if let Ok(proto_type) = ProtocolTypes::try_from(&*config_name) {
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
              match protocol::try_create_protocol(&proto_type, &*device_impl, device_protocol_config).await {
                Ok(protocol_impl) => Ok(Some(ButtplugDevice::new(protocol_impl, device_impl))),
                Err(e) => Err(e),
              }
            }
            Err(e) => Err(e),
          }
        } else {
          Ok(None)
        }
      }
      None => Ok(None),
    }
  }

  pub fn name(&self) -> &str {
    self.protocol.name()
  }

  pub fn message_attributes(&self) -> MessageAttributesMap {
    self.protocol.message_attributes()
  }

  pub fn parse_message(
    &self,
    message: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugServerResultFuture {
    self.protocol.handle_command(&*self.device, message)
  }
  
  // TODO Just return the receiver as part of the constructor
  pub fn get_event_receiver(&self) -> BoundedDeviceEventBroadcaster {
    self.device.get_event_receiver()
  }
  
  // TODO Handle raw messages here.
}
