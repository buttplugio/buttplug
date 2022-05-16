
// TODO This shouldn't exist in hardware but we need to keep it here temporarily.
mod device;
pub mod communication;
pub use device::ServerDevice;

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
    configuration::{ProtocolCommunicationSpecifier, ProtocolDeviceConfiguration},
  },
};
use async_trait::async_trait;
use futures::future::BoxFuture;
use tokio::sync::broadcast;

/// Future that executes and returns a response from a client command or request
pub type ServerDeviceResultFuture =
  BoxFuture<'static, Result<ButtplugServerMessage, ButtplugError>>;

/// Parameters for reading data from a [Hardware](crate::device::Hardware) endpoint
/// 
/// Low level read command structure, used by
/// [ButtplugProtocol](crate::device::protocol::ButtplugProtocol) implementations when working with
/// [Hardware](crate::device::Hardware) structures.
#[derive(PartialEq, Debug)]
pub struct HardwareReadCmd {
  /// Endpoint to read from
  pub endpoint: Endpoint,
  /// Amount of data to read from endpoint
  pub length: u32,
  /// Timeout for reading data
  pub timeout_ms: u32,
}

impl HardwareReadCmd {
  /// Creates a new DeviceReadCmd instance
  pub fn new(endpoint: Endpoint, length: u32, timeout_ms: u32) -> Self {
    Self {
      endpoint,
      length,
      timeout_ms,
    }
  }
}

impl From<RawReadCmd> for HardwareReadCmd {
  fn from(msg: RawReadCmd) -> Self {
    Self {
      endpoint: msg.endpoint(),
      length: msg.expected_length(),
      timeout_ms: msg.timeout(),
    }
  }
}

/// Parameters for writing data to a [Hardware](crate::device::Hardware) endpoint
/// 
/// Low level write command structure, used by
/// [ButtplugProtocol](crate::device::protocol::ButtplugProtocol) implementations when working with
/// [Hardware](crate::device::Hardware) structures.
#[derive(PartialEq, Debug)]
pub struct HardwareWriteCmd {
  /// Endpoint to write to
  pub endpoint: Endpoint,
  /// Data to write to endpoint
  pub data: Vec<u8>,
  /// Only used with Bluetooth LE writing. If true, use WriteWithResponse commands when sending data to device.
  pub write_with_response: bool,
}

impl HardwareWriteCmd {
  /// Create a new DeviceWriteCmd instance.
  pub fn new(endpoint: Endpoint, data: Vec<u8>, write_with_response: bool) -> Self {
    Self {
      endpoint,
      data,
      write_with_response,
    }
  }
}

impl From<RawWriteCmd> for HardwareWriteCmd {
  fn from(msg: RawWriteCmd) -> Self {
    Self {
      endpoint: msg.endpoint(),
      data: msg.data().clone(),
      write_with_response: msg.write_with_response(),
    }
  }
}

/// Parameters for subscribing to a [Hardware](crate::device::Hardware) endpoint
/// 
/// Low level subscribe structure, used by
/// [ButtplugProtocol](crate::device::protocol::ButtplugProtocol) implementations when working with
/// [Hardware](crate::device::Hardware) structures.
/// 
/// While usually related to notify/indicate characteristics on Bluetooth LE devices, can be used
/// with any read endpoint to signal that any information received should be automatically passed to
/// the protocol implementation.
#[derive(PartialEq, Debug)]
pub struct HardwareSubscribeCmd {
  /// Endpoint to subscribe to notifications from.
  pub endpoint: Endpoint,
}

impl HardwareSubscribeCmd {
  /// Create a new DeviceSubscribeCmd instance
  pub fn new(endpoint: Endpoint) -> Self {
    Self { endpoint }
  }
}

impl From<RawSubscribeCmd> for HardwareSubscribeCmd {
  fn from(msg: RawSubscribeCmd) -> Self {
    Self {
      endpoint: msg.endpoint(),
    }
  }
}

/// Parameters for unsubscribing from a [Hardware](crate::device::Hardware) endpoint that has
/// previously been subscribed.
/// 
/// Low level subscribe structure, used by
/// [ButtplugProtocol](crate::device::protocol::ButtplugProtocol) implementations when working with
/// [Hardware](crate::device::Hardware) structures.
#[derive(PartialEq, Debug)]
pub struct HardwareUnsubscribeCmd {
  pub endpoint: Endpoint,
}

impl HardwareUnsubscribeCmd {
  /// Create a new DeviceUnsubscribeCmd instance
  pub fn new(endpoint: Endpoint) -> Self {
    Self { endpoint }
  }
}

impl From<RawUnsubscribeCmd> for HardwareUnsubscribeCmd {
  fn from(msg: RawUnsubscribeCmd) -> Self {
    Self {
      endpoint: msg.endpoint(),
    }
  }
}

/// Enumeration of all possible commands that can be sent to a
/// [Hardware](crate::device::Hardware).
#[derive(PartialEq, Debug)]
pub enum HardwareCommand {
  Write(HardwareWriteCmd),
  Read(HardwareReadCmd),
  Subscribe(HardwareSubscribeCmd),
  Unsubscribe(HardwareUnsubscribeCmd),
}

impl From<RawWriteCmd> for HardwareCommand {
  fn from(msg: RawWriteCmd) -> Self {
    HardwareCommand::Write(msg.into())
  }
}

impl From<RawSubscribeCmd> for HardwareCommand {
  fn from(msg: RawSubscribeCmd) -> Self {
    HardwareCommand::Subscribe(msg.into())
  }
}

impl From<RawUnsubscribeCmd> for HardwareCommand {
  fn from(msg: RawUnsubscribeCmd) -> Self {
    HardwareCommand::Unsubscribe(msg.into())
  }
}

impl From<HardwareReadCmd> for HardwareCommand {
  fn from(msg: HardwareReadCmd) -> Self {
    HardwareCommand::Read(msg)
  }
}

impl From<HardwareWriteCmd> for HardwareCommand {
  fn from(msg: HardwareWriteCmd) -> Self {
    HardwareCommand::Write(msg)
  }
}

impl From<HardwareSubscribeCmd> for HardwareCommand {
  fn from(msg: HardwareSubscribeCmd) -> Self {
    HardwareCommand::Subscribe(msg)
  }
}

impl From<HardwareUnsubscribeCmd> for HardwareCommand {
  fn from(msg: HardwareUnsubscribeCmd) -> Self {
    HardwareCommand::Unsubscribe(msg)
  }
}

/// Events that can be emitted from a [Hardware](crate::device::Hardware).
#[derive(Debug, Clone)]
pub enum HardwareEvent {
  /// Device connected
  Connected(Arc<ServerDevice>),
  /// Device received data
  Notification(String, Endpoint, Vec<u8>),
  /// Device disconnected
  Disconnected(String),
}

/// Hardware implementation and communication portion of a
/// [ButtplugDevice](crate::device::ButtplugDevice) instance. The Hardware contains a
/// HardwareInternal, which handles all of the actual hardware communication. However, the struct
/// also needs to carry around identifying information, so we wrap it in this type instead of
/// requiring that all implementors of deal with name/address/endpoint accessors.
pub struct Hardware {
  /// Device name
  name: String,
  /// Device address
  address: String,
  /// Communication endpoints
  endpoints: Vec<Endpoint>,
  /// Internal implementation details
  internal_impl: Box<dyn HardwareInternal>,
}

impl Hardware {
  pub fn new(
    name: &str,
    address: &str,
    endpoints: &[Endpoint],
    internal_impl: Box<dyn HardwareInternal>,
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
  pub fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
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
    msg: HardwareReadCmd,
  ) -> BoxFuture<'static, Result<RawReading, ButtplugError>> {
    self.internal_impl.read_value(msg)
  }

  /// Write a value to the device
  pub fn write_value(&self, msg: HardwareWriteCmd) -> ButtplugResultFuture {
    self.internal_impl.write_value(msg)
  }

  /// Subscribe to a device endpoint, if it exists
  pub fn subscribe(&self, msg: HardwareSubscribeCmd) -> ButtplugResultFuture {
    self.internal_impl.subscribe(msg)
  }

  /// Unsubscribe from a device endpoint, if it exists
  pub fn unsubscribe(&self, msg: HardwareUnsubscribeCmd) -> ButtplugResultFuture {
    self.internal_impl.unsubscribe(msg)
  }
}

/// Internal representation of device implementations
/// 
/// This trait is implemented by
/// [DeviceCommunicationManager](crate::server::device::communication_manager::DeviceCommunicationManager) modules
/// to represent and communicate with devices. It provides an abstract way to represent devices
/// without having to consider what type of communication bus they may be using.
pub trait HardwareInternal: Sync + Send {
  /// If true, device is currently connected to system
  fn connected(&self) -> bool;
  /// Disconnect from the device (if it is connected)
  fn disconnect(&self) -> ButtplugResultFuture;
  /// Returns a receiver for any events the device may emit.
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent>;
  /// Read a value from the device
  fn read_value(&self, msg: HardwareReadCmd)
    -> BoxFuture<'static, Result<RawReading, ButtplugError>>;
  /// Write a value to the device
  fn write_value(&self, msg: HardwareWriteCmd) -> ButtplugResultFuture;
  /// Subscribe to a device endpoint, if it exists
  fn subscribe(&self, msg: HardwareSubscribeCmd) -> ButtplugResultFuture;
  /// Unsubscribe from a device endpoint, if it exists
  fn unsubscribe(&self, msg: HardwareUnsubscribeCmd) -> ButtplugResultFuture;
}

/// Factory trait for [Hardware](crate::device::Hardware) instances in
/// [DeviceCommunicationManager](crate::server::device::communication_manager::DeviceCommunicationManager) modules
/// 
/// This trait is implemented by
/// [DeviceCommunicationManager](crate::server::device::communication_manager::DeviceCommunicationManager) modules
/// to handle initial device connection and setup based on the specific communication bus that is
/// being implemented by the DCM. This may handle things like connection and finding characteristics
/// for Bluetooth LE, connection to USB devices and checking descriptors/endpoints, etc...
/// 
/// If a [Hardware](crate::device::Hardware) is returned from the try_create_hardware method,
/// it is assumed the device is connected and ready to be used.
#[async_trait]
pub trait HardwareCreator: Sync + Send + Debug {
  /// Return the hardware identifier for the device. Depends on the communication bus type, so may
  /// be a bluetooth name, serial port name, etc...
  fn specifier(&self) -> ProtocolCommunicationSpecifier;
  /// Try to connect to and initialize a device.
  /// 
  /// Given a
  /// [ProtocolDeviceConfiguration](crate::server::device::configuration::ProtocolDeviceConfiguration)
  /// which will contain information about what a protocol needs to communicate with a device, try
  /// to connect to the device and identify all required endpoints.
  async fn try_create_hardware(
    &mut self,
    protocol: ProtocolDeviceConfiguration,
  ) -> Result<Hardware, ButtplugError>;
}
