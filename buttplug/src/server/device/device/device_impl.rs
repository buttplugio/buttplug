use std::{
  fmt::Debug,
  sync::Arc,
};

use crate::{
  core::{
    errors::ButtplugError,
    messages::{
      ButtplugServerMessage,
      Endpoint,
      RawReadCmd,
      RawReading,
      RawSubscribeCmd,
      RawUnsubscribeCmd,
      RawWriteCmd,
    },
    ButtplugResultFuture,
  },
  server::device::{
    device::{ButtplugDevice},
    configuration::{ProtocolCommunicationSpecifier, ProtocolDeviceConfiguration},
  },
};
use async_trait::async_trait;
use futures::future::BoxFuture;
use tokio::sync::broadcast;

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
  /// [ProtocolDeviceConfiguration](crate::server::device::configuration::ProtocolDeviceConfiguration)
  /// which will contain information about what a protocol needs to communicate with a device, try
  /// to connect to the device and identify all required endpoints.
  async fn try_create_device_impl(
    &mut self,
    protocol: ProtocolDeviceConfiguration,
  ) -> Result<DeviceImpl, ButtplugError>;
}
