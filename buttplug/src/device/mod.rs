// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.


//! Device identification and configuration, and protocol definitions
//!
//! Structs in the device module are used by the [Buttplug Server](crate::server) (specifically
//! the [Device Manager](crate::server::device_manager::DeviceManager)) to identify devices that
//! Buttplug can connect to, and match them to supported protocols in order to establish
//! communication. 

pub mod configuration_manager;
pub mod protocol;
use once_cell::sync::OnceCell;
use serde::{
  de::{self, Visitor},
  Deserialize,
  Deserializer,
  Serialize,
  Serializer,
};
use std::{
  fmt::{self, Debug},
  str::FromStr,
  string::ToString,
  sync::Arc,
};

use crate::{
  core::{
    errors::ButtplugError,
    messages::{
      ButtplugDeviceCommandMessageUnion,
      ButtplugServerMessage,
      DeviceMessageAttributesMap,
      RawReadCmd,
      RawReading,
      RawSubscribeCmd,
      RawUnsubscribeCmd,
      RawWriteCmd,
    },
    ButtplugResultFuture,
  },
  device::{
    configuration_manager::{DeviceConfigurationManager, ProtocolCommunicationSpecifier, ProtocolDeviceConfiguration, ProtocolAttributesIdentifier, ProtocolDeviceIdentifier},
    protocol::ButtplugProtocol,
  },
};
use async_trait::async_trait;
use core::hash::{Hash, Hasher};
use futures::future::BoxFuture;
use tokio::sync::broadcast;



// We need this array to be exposed in our WASM FFI, but the only way to do that
// is to expose it at the declaration level. Therefore, we use the WASM feature
// to assume we're building for WASM and attach our bindgen. The serde
// de/serialization is taken care of at the FFI level.
/// Endpoint names for device communication.
/// 
/// Endpoints denote different contextual communication targets on a device. For instance, for a
/// device that uses UART style communication (serial, a lot of Bluetooth LE devices, etc...) most
/// devices will just have a Tx and Rx endpoint. However, on other devices that can have varying
/// numbers of endpoints and configurations (USB, Bluetooth LE, etc...) we add some names with more
/// context. These names are used in [Device Configuration](crate::device::configuration_manager)
/// and the [Device Configuration File](crate::util::device_configuration), and are expected to
/// de/serialize to lowercase versions of their names.
#[derive(EnumString, Clone, Debug, PartialEq, Eq, Hash, Display, Copy)]
#[strum(serialize_all = "lowercase")]
pub enum Endpoint {
  /// Expect to take commands, when multiple receive endpoints may be available
  Command,
  /// Firmware updates (Buttplug does not update firmware, but some firmware endpoints are used for
  /// mode setting)
  Firmware,
  /// Common receive endpoint name
  Rx,
  /// Receive endpoint for accelerometer data
  RxAccel,
  /// Receive endpoint for battery levels (usually expected to be BLE standard profile)
  RxBLEBattery,
  /// Receive endpoint for BLE model (usually expected to be BLE standard profile)
  RxBLEModel,
  /// Receive endpoint for pressure sensors
  RxPressure,
  /// Receive endpoint for touch sensors
  RxTouch,
  /// Common transmit endpoint name
  Tx,
  /// Transmit endpoint for hardware mode setting.
  TxMode,
  /// Transmit endpoint for shock setting (unused)
  TxShock,
  /// Transmit endpoint for vibration setting
  TxVibrate,
  /// Transmit endpoint for vendor (proprietary) control
  TxVendorControl,
  /// Transmit endpoint for whitelist updating
  Whitelist,
  /// Generic endpoint (available for user configurations)
  Generic0,
  /// Generic endpoint (available for user configurations)
  Generic1,
  /// Generic endpoint (available for user configurations)
  Generic2,
  /// Generic endpoint (available for user configurations)
  Generic3,
  /// Generic endpoint (available for user configurations)
  Generic4,
  /// Generic endpoint (available for user configurations)
  Generic5,
  /// Generic endpoint (available for user configurations)
  Generic6,
  /// Generic endpoint (available for user configurations)
  Generic7,
  /// Generic endpoint (available for user configurations)
  Generic8,
  /// Generic endpoint (available for user configurations)
  Generic9,
  /// Generic endpoint (available for user configurations)
  Generic10,
  /// Generic endpoint (available for user configurations)
  Generic11,
  /// Generic endpoint (available for user configurations)
  Generic12,
  /// Generic endpoint (available for user configurations)
  Generic13,
  /// Generic endpoint (available for user configurations)
  Generic14,
  /// Generic endpoint (available for user configurations)
  Generic15,
  /// Generic endpoint (available for user configurations)
  Generic16,
  /// Generic endpoint (available for user configurations)
  Generic17,
  /// Generic endpoint (available for user configurations)
  Generic18,
  /// Generic endpoint (available for user configurations)
  Generic19,
  /// Generic endpoint (available for user configurations)
  Generic20,
  /// Generic endpoint (available for user configurations)
  Generic21,
  /// Generic endpoint (available for user configurations)
  Generic22,
  /// Generic endpoint (available for user configurations)
  Generic23,
  /// Generic endpoint (available for user configurations)
  Generic24,
  /// Generic endpoint (available for user configurations)
  Generic25,
  /// Generic endpoint (available for user configurations)
  Generic26,
  /// Generic endpoint (available for user configurations)
  Generic27,
  /// Generic endpoint (available for user configurations)
  Generic28,
  /// Generic endpoint (available for user configurations)
  Generic29,
  /// Generic endpoint (available for user configurations)
  Generic30,
  /// Generic endpoint (available for user configurations)
  Generic31,
}

// Implement to/from string serialization for Endpoint struct
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

/// Future that executes and returns a response from a client command or request
pub type ButtplugDeviceResultFuture =
  BoxFuture<'static, Result<ButtplugServerMessage, ButtplugError>>;

/// Parameters for reading data from a [DeviceImpl](crate::device::DeviceImpl) endpoint
/// 
/// Low level read command structure, used by
/// [ButtplugProtocol](crate::device::protocol::ButtplugProtocol) implementations when working with
/// [DeviceImpl](crate::device::DeviceImpl) structures.
#[derive(PartialEq, Debug)]
pub struct DeviceReadCmd {
  /// Endpoint to read from
  pub endpoint: Endpoint,
  /// Amount of data to read from endpoint
  pub length: u32,
  /// Timeout for reading data
  pub timeout_ms: u32,
}

impl DeviceReadCmd {
  /// Creates a new DeviceReadCmd instance
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
      endpoint: msg.endpoint(),
      length: msg.expected_length(),
      timeout_ms: msg.timeout(),
    }
  }
}

/// Parameters for writing data to a [DeviceImpl](crate::device::DeviceImpl) endpoint
/// 
/// Low level write command structure, used by
/// [ButtplugProtocol](crate::device::protocol::ButtplugProtocol) implementations when working with
/// [DeviceImpl](crate::device::DeviceImpl) structures.
#[derive(PartialEq, Debug)]
pub struct DeviceWriteCmd {
  /// Endpoint to write to
  pub endpoint: Endpoint,
  /// Data to write to endpoint
  pub data: Vec<u8>,
  /// Only used with Bluetooth LE writing. If true, use WriteWithResponse commands when sending data to device.
  pub write_with_response: bool,
}

impl DeviceWriteCmd {
  /// Create a new DeviceWriteCmd instance.
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
      endpoint: msg.endpoint(),
      data: msg.data().clone(),
      write_with_response: msg.write_with_response(),
    }
  }
}

/// Parameters for subscribing to a [DeviceImpl](crate::device::DeviceImpl) endpoint
/// 
/// Low level subscribe structure, used by
/// [ButtplugProtocol](crate::device::protocol::ButtplugProtocol) implementations when working with
/// [DeviceImpl](crate::device::DeviceImpl) structures.
/// 
/// While usually related to notify/indicate characteristics on Bluetooth LE devices, can be used
/// with any read endpoint to signal that any information received should be automatically passed to
/// the protocol implementation.
#[derive(PartialEq, Debug)]
pub struct DeviceSubscribeCmd {
  /// Endpoint to subscribe to notifications from.
  pub endpoint: Endpoint,
}

impl DeviceSubscribeCmd {
  /// Create a new DeviceSubscribeCmd instance
  pub fn new(endpoint: Endpoint) -> Self {
    Self { endpoint }
  }
}

impl From<RawSubscribeCmd> for DeviceSubscribeCmd {
  fn from(msg: RawSubscribeCmd) -> Self {
    Self {
      endpoint: msg.endpoint(),
    }
  }
}

/// Parameters for unsubscribing from a [DeviceImpl](crate::device::DeviceImpl) endpoint that has
/// previously been subscribed.
/// 
/// Low level subscribe structure, used by
/// [ButtplugProtocol](crate::device::protocol::ButtplugProtocol) implementations when working with
/// [DeviceImpl](crate::device::DeviceImpl) structures.
#[derive(PartialEq, Debug)]
pub struct DeviceUnsubscribeCmd {
  pub endpoint: Endpoint,
}

impl DeviceUnsubscribeCmd {
  /// Create a new DeviceUnsubscribeCmd instance
  pub fn new(endpoint: Endpoint) -> Self {
    Self { endpoint }
  }
}

impl From<RawUnsubscribeCmd> for DeviceUnsubscribeCmd {
  fn from(msg: RawUnsubscribeCmd) -> Self {
    Self {
      endpoint: msg.endpoint(),
    }
  }
}

/// Enumeration of all possible commands that can be sent to a
/// [DeviceImpl](crate::device::DeviceImpl).
#[derive(PartialEq, Debug)]
pub enum DeviceImplCommand {
  Write(DeviceWriteCmd),
  Read(DeviceReadCmd),
  Subscribe(DeviceSubscribeCmd),
  Unsubscribe(DeviceUnsubscribeCmd),
}

impl From<RawWriteCmd> for DeviceImplCommand {
  fn from(msg: RawWriteCmd) -> Self {
    DeviceImplCommand::Write(msg.into())
  }
}

impl From<RawSubscribeCmd> for DeviceImplCommand {
  fn from(msg: RawSubscribeCmd) -> Self {
    DeviceImplCommand::Subscribe(msg.into())
  }
}

impl From<RawUnsubscribeCmd> for DeviceImplCommand {
  fn from(msg: RawUnsubscribeCmd) -> Self {
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

/// Events that can be emitted from a [DeviceImpl](crate::device::DeviceImpl).
#[derive(Debug, Clone)]
pub enum ButtplugDeviceEvent {
  /// Device connected
  Connected(Arc<ButtplugDevice>),
  /// Device received data
  Notification(String, Endpoint, Vec<u8>),
  /// Device disconnected
  Disconnected(String),
}

/// Hardware implementation and communication portion of a
/// [ButtplugDevice](crate::device::ButtplugDevice) instance.
pub struct DeviceImpl {
  /// Device name
  name: String,
  /// Device address
  address: String,
  /// Communication endpoints
  endpoints: Vec<Endpoint>,
  /// Internal implementation details
  internal_impl: Box<dyn DeviceImplInternal>,
}

impl DeviceImpl {
  pub fn new(
    name: &str,
    address: &str,
    endpoints: &[Endpoint],
    internal_impl: Box<dyn DeviceImplInternal>,
  ) -> Self {
    Self {
      name: name.to_owned(),
      address: address.to_owned(),
      endpoints: endpoints.into(),
      internal_impl,
    }
  }

  /// Returns the device name
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Returns the device address
  pub fn address(&self) -> &str {
    &self.address
  }

  /// If true, device is currently connected to system
  pub fn connected(&self) -> bool {
    self.internal_impl.connected()
  }

  /// Returns a receiver for any events the device may emit.
  /// 
  /// This uses a broadcast channel and can be called multiple times to create multiple streams if
  /// needed.
  pub fn event_stream(&self) -> broadcast::Receiver<ButtplugDeviceEvent> {
    self.internal_impl.event_stream()
  }

  /// Returns the device endpoint list
  pub fn endpoints(&self) -> Vec<Endpoint> {
    self.endpoints.clone()
  }

  /// Disconnect from the device (if it is connected)
  pub fn disconnect(&self) -> ButtplugResultFuture {
    self.internal_impl.disconnect()
  }

  /// Read a value from the device
  pub fn read_value(
    &self,
    msg: DeviceReadCmd,
  ) -> BoxFuture<'static, Result<RawReading, ButtplugError>> {
    self.internal_impl.read_value(msg)
  }

  /// Write a value to the device
  pub fn write_value(&self, msg: DeviceWriteCmd) -> ButtplugResultFuture {
    self.internal_impl.write_value(msg)
  }

  /// Subscribe to a device endpoint, if it exists
  pub fn subscribe(&self, msg: DeviceSubscribeCmd) -> ButtplugResultFuture {
    self.internal_impl.subscribe(msg)
  }

  /// Unsubscribe from a device endpoint, if it exists
  pub fn unsubscribe(&self, msg: DeviceUnsubscribeCmd) -> ButtplugResultFuture {
    self.internal_impl.unsubscribe(msg)
  }
}

/// Internal representation of device implementations
/// 
/// This trait is implemented by
/// [DeviceCommunicationManager](crate::server::device::communication_manager::DeviceCommunicationManager) modules
/// to represent and communicate with devices. It provides an abstract way to represent devices
/// without having to consider what type of communication bus they may be using.
pub trait DeviceImplInternal: Sync + Send {
  /// If true, device is currently connected to system
  fn connected(&self) -> bool;
  /// Disconnect from the device (if it is connected)
  fn disconnect(&self) -> ButtplugResultFuture;
  /// Returns a receiver for any events the device may emit.
  fn event_stream(&self) -> broadcast::Receiver<ButtplugDeviceEvent>;
  /// Read a value from the device
  fn read_value(&self, msg: DeviceReadCmd)
    -> BoxFuture<'static, Result<RawReading, ButtplugError>>;
  /// Write a value to the device
  fn write_value(&self, msg: DeviceWriteCmd) -> ButtplugResultFuture;
  /// Subscribe to a device endpoint, if it exists
  fn subscribe(&self, msg: DeviceSubscribeCmd) -> ButtplugResultFuture;
  /// Unsubscribe from a device endpoint, if it exists
  fn unsubscribe(&self, msg: DeviceUnsubscribeCmd) -> ButtplugResultFuture;
}

/// Factory trait for [DeviceImpl](crate::device::DeviceImpl) instances in
/// [DeviceCommunicationManager](crate::server::device::communication_manager::DeviceCommunicationManager) modules
/// 
/// This trait is implemented by
/// [DeviceCommunicationManager](crate::server::device::communication_manager::DeviceCommunicationManager) modules
/// to handle initial device connection and setup based on the specific communication bus that is
/// being implemented by the DCM. This may handle things like connection and finding characteristics
/// for Bluetooth LE, connection to USB devices and checking descriptors/endpoints, etc...
/// 
/// If a [DeviceImpl](crate::device::DeviceImpl) is returned from the try_create_device_impl method,
/// it is assumed the device is connected and ready to be used.
#[async_trait]
pub trait ButtplugDeviceImplCreator: Sync + Send + Debug {
  /// Return the hardware identifier for the device. Depends on the communication bus type, so may
  /// be a bluetooth name, serial port name, etc...
  fn specifier(&self) -> ProtocolCommunicationSpecifier;
  /// Try to connect to and initialize a device.
  /// 
  /// Given a
  /// [ProtocolDeviceConfiguration](crate::device::configuration_manager::ProtocolDeviceConfiguration)
  /// which will contain information about what a protocol needs to communicate with a device, try
  /// to connect to the device and identify all required endpoints.
  async fn try_create_device_impl(
    &mut self,
    protocol: ProtocolDeviceConfiguration,
  ) -> Result<DeviceImpl, ButtplugError>;
}

/// Main internal device representation structure
/// 
/// A ButtplugDevice is made up of 2 components:
/// 
/// - A [Device Implementation](crate::device::DeviceImpl), which handles hardware connection and
///   communication.
/// - A [Protocol](crate::device::protocol::ButtplugProtocol), which takes Buttplug Commands and
///   translated them into propreitary commands to send to a device.
/// 
/// When a ButtplugDevice instance is created, it can be assumed that it represents a device that is
/// connected and has been successfully initialized. The instance will manage communication of all
/// commands sent from a [client](crate::client::ButtplugClient) that pertain to this specific
/// hardware.
pub struct ButtplugDevice {
  /// Protocol instance for the device
  protocol: Box<dyn ButtplugProtocol>,
  /// Hardware implementation for the device
  device: Arc<DeviceImpl>,
  /// Display name for the device
  display_name: OnceCell<String>,
  /// Unique identifier for the device
  device_identifier: ProtocolDeviceIdentifier
}

impl Debug for ButtplugDevice {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ButtplugDevice")
      .field("name", &self.name())
      .field("identifier", &self.device_identifier())
      .finish()
  }
}

impl Hash for ButtplugDevice {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.device_identifier().hash(state);
  }
}

impl Eq for ButtplugDevice {
}

impl PartialEq for ButtplugDevice {
  fn eq(&self, other: &Self) -> bool {
    self.device_identifier() == other.device_identifier()
  }
}

impl ButtplugDevice {
  /// Given a protocol and a device impl, create a new ButtplugDevice instance
  fn new(protocol: Box<dyn ButtplugProtocol>, device: Arc<DeviceImpl>) -> Self {
    Self {
      device_identifier: ProtocolDeviceIdentifier::new(device.address(), protocol.protocol_identifier(), protocol.protocol_attributes_identifier()),
      protocol,
      device,
      display_name: OnceCell::new(),
    }
  }

  /// Returns the device identifier
  pub fn device_identifier(&self) -> &ProtocolDeviceIdentifier {
    &self.device_identifier
  }

  /// Returns the address of the device implementation
  pub fn device_impl_address(&self) -> &str {
    self.device.address()
  }

  /// Returns the protocol identifier
  pub fn protocol_identifier(&self) -> &str {
    self.protocol.protocol_identifier()
  }

  /// Returns the protocol attribute identifier
  pub fn protocol_attributes_identifier(&self) -> &ProtocolAttributesIdentifier {
    self.protocol.protocol_attributes_identifier()
  }

  /// Given a possibly usable device, see if any protocol matches. If so, connect and initialize.
  /// 
  /// This is called any time we get a device detection or advertisement from one of our
  /// DeviceCommunicationManager instances. This could be anything from a Bluetooth advertisement,
  /// to detection of a USB device, to one of the various network systems declaring they've found
  /// something. Given the information we've received from that, plus our
  /// [DeviceConfigurationManager](crate::device::configuration_manager::DeviceConfigurationManager),
  /// try to find a protocol that has information matching this device. This may include name match,
  /// port matching, etc...
  /// 
  /// If a matching protocol is found, we then call
  /// [ButtplugDeviceImplCreator::try_create_device_impl](crate::device::ButtplugDeviceImplCreator::try_create_device_impl)
  /// with the related protocol information, in order to connect and initialize the device.
  /// 
  /// If all of that is successful, we return a ButtplugDevice that is ready to advertise to the
  /// client and use.
  pub async fn try_create_device(
    device_config_mgr: Arc<DeviceConfigurationManager>,
    mut device_creator: Box<dyn ButtplugDeviceImplCreator>,
  ) -> Result<Option<ButtplugDevice>, ButtplugError> {
    // TODO This seems needlessly complex, can we clean up how we pass the device builder and protocol factory around?
    
    // First off, we need to see if we even have a configuration available for the device we're
    // trying to create. If we don't, return Ok(None), because this isn't actually an error.
    // However, if we *do* have a configuration but something goes wrong after this, then it's an
    // error.
    let protocol_builder = match device_config_mgr.protocol_instance_factory(&device_creator.specifier()) {
      Some(builder) => builder,
      None => return Ok(None)
    };
      

    // Now that we have both a possible device implementation and a configuration for that device,
    // try to initialize the implementation. This usually means trying to connect to whatever the
    // device is, finding endpoints, etc.
    let device_impl = device_creator.try_create_device_impl(protocol_builder.configuration().clone()).await?;
    info!(
      address = tracing::field::display(device_impl.address()),
      "Found Buttplug Device {}",
      device_impl.name()
    );

    // If we've made it this far, we now have a connected device implementation with endpoints set
    // up. We now need to run whatever protocol initialization might need to happen. We'll fetch a
    // protocol creator, pass the device implementation to it, then let it do whatever it needs. For
    // most protocols, this is a no-op. However, for devices like Lovense, some Kiiroo, etc, this
    // can get fairly complicated.
    let sharable_device_impl = Arc::new(device_impl);
    let protocol_impl =
      protocol_builder.create(sharable_device_impl.clone()).await?;
    Ok(Some(ButtplugDevice::new(
      protocol_impl,
      sharable_device_impl,
    )))
  }

  /// Get the user created display name for a device, if one exists.
  pub fn display_name(&self) -> Option<String> {
    self.display_name.get().and_then(|name| Some(name.clone()))
  }

  /// Get the name of the device as set in the Device Configuration File.
  /// 
  /// This will also append "(Raw Messaged Allowed)" to the device name if raw mode is on, to warn
  /// users that the device is capable of direct communication.
  pub fn name(&self) -> String {
    // Instead of checking for raw messages at the protocol level, add the raw
    // call here, since this is the only way to access devices in the library
    // anyways.
    //
    // Having raw turned on means it'll work for read/write/sub/unsub on any
    // endpoint so just use an arbitrary message here to check.
    if self
      .protocol
      .supports_message(&ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(
        RawSubscribeCmd::new(1, Endpoint::Tx),
      ))
      .is_ok()
    {
      format!("{} (Raw Messages Allowed)", self.protocol.name())
    } else {
      self.protocol.name().to_owned()
    }
  }

  /// Disconnect from the device, if it's connected.
  pub fn disconnect(&self) -> ButtplugResultFuture {
    self.device.disconnect()
  }

  /// Retreive the message attributes for the device.
  pub fn message_attributes(&self) -> DeviceMessageAttributesMap {
    self.protocol.device_attributes().message_attributes_map()
  }

  /// Parse and route a client command sent for this device.
  pub fn parse_message(
    &self,
    message: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugDeviceResultFuture {
    self.protocol.handle_command(self.device.clone(), message)
  }

  /// Retreive the event stream for the device.
  /// 
  /// This will include connections, disconnections, and notification events from subscribed
  /// endpoints.
  pub fn event_stream(&self) -> broadcast::Receiver<ButtplugDeviceEvent> {
    self.device.event_stream()
  }
}
