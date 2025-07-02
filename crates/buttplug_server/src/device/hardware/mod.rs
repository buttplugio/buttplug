pub mod communication;
use std::{collections::HashSet, fmt::Debug, sync::Arc, time::Duration};

use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::{ProtocolCommunicationSpecifier, Endpoint};
use futures::future::BoxFuture;
use futures_util::FutureExt;
use getset::{CopyGetters, Getters};
use instant::Instant;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

/// Parameters for reading data from a [Hardware](crate::device::Hardware) endpoint
///
/// Low level read command structure, used by
/// [ButtplugProtocol](crate::device::protocol::ButtplugProtocol) implementations when working with
/// [Hardware](crate::device::Hardware) structures.
#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct HardwareReadCmd {
  /// Feature ID for reading
  #[serde(default)]
  command_id: Uuid,
  /// Endpoint to read from
  endpoint: Endpoint,
  /// Amount of data to read from endpoint
  length: u32,
  /// Timeout for reading data
  timeout_ms: u32,
}

impl HardwareReadCmd {
  /// Creates a new DeviceReadCmd instance
  pub fn new(command_id: Uuid, endpoint: Endpoint, length: u32, timeout_ms: u32) -> Self {
    Self {
      command_id,
      endpoint,
      length,
      timeout_ms,
    }
  }
}

/// Parameters for writing data to a [Hardware](crate::device::Hardware) endpoint
///
/// Low level write command structure, used by
/// [ButtplugProtocol](crate::device::protocol::ButtplugProtocol) implementations when working with
/// [Hardware](crate::device::Hardware) structures.
#[derive(Eq, Debug, Clone, Serialize, Deserialize, Getters, CopyGetters)]
pub struct HardwareWriteCmd {
  /// Feature ID for this command. As a write command can possibly write to multiple features in one
  /// call, this can have multiple feature IDs.
  #[getset(get = "pub")]
  #[serde(default)]
  command_id: HashSet<Uuid>,
  /// Endpoint to write to
  #[getset(get_copy = "pub")]
  endpoint: Endpoint,
  /// Data to write to endpoint
  #[getset(get = "pub")]
  data: Vec<u8>,
  /// Only used with Bluetooth LE writing. If true, use WriteWithResponse commands when sending data to device.
  #[getset(get_copy = "pub")]
  write_with_response: bool,
}

impl PartialEq for HardwareWriteCmd {
  fn eq(&self, other: &Self) -> bool {
    self.endpoint() == other.endpoint()
      && self.data() == other.data()
      && self.write_with_response() == other.write_with_response()
  }
}

impl HardwareWriteCmd {
  /// Create a new DeviceWriteCmd instance.
  pub fn new(
    command_id: &[Uuid],
    endpoint: Endpoint,
    data: Vec<u8>,
    write_with_response: bool,
  ) -> Self {
    Self {
      command_id: HashSet::from_iter(command_id.iter().cloned()),
      endpoint,
      data,
      write_with_response,
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
#[derive(Eq, Debug, Clone, Copy, Serialize, Deserialize, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct HardwareSubscribeCmd {
  /// Feature ID for this command
  #[getset(get_copy = "pub")]
  #[serde(default)]
  command_id: Uuid,
  /// Endpoint to subscribe to notifications from.
  endpoint: Endpoint,
}

impl PartialEq for HardwareSubscribeCmd {
  fn eq(&self, other: &Self) -> bool {
    self.endpoint() == other.endpoint()
  }
}

impl HardwareSubscribeCmd {
  /// Create a new DeviceSubscribeCmd instance
  pub fn new(command_id: Uuid, endpoint: Endpoint) -> Self {
    Self {
      command_id,
      endpoint,
    }
  }
}

/// Parameters for unsubscribing from a [Hardware](crate::device::Hardware) endpoint that has
/// previously been subscribed.
///
/// Low level subscribe structure, used by
/// [ButtplugProtocol](crate::device::protocol::ButtplugProtocol) implementations when working with
/// [Hardware](crate::device::Hardware) structures.
#[derive(Eq, Debug, Clone, Copy, Serialize, Deserialize, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct HardwareUnsubscribeCmd {
  #[serde(default)]
  command_id: Uuid,
  endpoint: Endpoint,
}

impl PartialEq for HardwareUnsubscribeCmd {
  fn eq(&self, other: &Self) -> bool {
    self.endpoint() == other.endpoint()
  }
}

impl HardwareUnsubscribeCmd {
  /// Create a new DeviceUnsubscribeCmd instance
  pub fn new(command_id: Uuid, endpoint: Endpoint) -> Self {
    Self {
      command_id,
      endpoint,
    }
  }
}

/// Enumeration of all possible commands that can be sent to a
/// [Hardware](crate::device::Hardware).
#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum HardwareCommand {
  Write(HardwareWriteCmd),
  // Read not included here because it needs to be called directly so the response can be handled.
  Subscribe(HardwareSubscribeCmd),
  Unsubscribe(HardwareUnsubscribeCmd),
}

impl HardwareCommand {
  pub fn overlaps(&self, command: &HardwareCommand) -> bool {
    // There is probably a cleaner way to write these match branches to drop the if/else and default
    // out to false, but I can't figure it out right now.
    match self {
      HardwareCommand::Write(c) => {
        if let HardwareCommand::Write(write) = command {
          c.command_id().intersection(&write.command_id).count() > 0
        } else {
          false
        }
      }
      HardwareCommand::Subscribe(c) => {
        if let HardwareCommand::Subscribe(sub) = command {
          c.command_id() == sub.command_id
        } else {
          false
        }
      }
      HardwareCommand::Unsubscribe(c) => {
        if let HardwareCommand::Unsubscribe(sub) = command {
          c.command_id() == sub.command_id
        } else {
          false
        }
      }
    }
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

#[derive(Debug, Clone, Getters)]
#[getset(get = "pub")]
pub struct HardwareReading {
  endpoint: Endpoint,
  data: Vec<u8>,
}

impl HardwareReading {
  pub fn new(endpoint: Endpoint, data: &[u8]) -> Self {
    Self {
      endpoint,
      data: data.to_vec(),
    }
  }
}

/// Events that can be emitted from a [Hardware](crate::device::Hardware).
#[derive(Debug, Clone)]
pub enum HardwareEvent {
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
#[derive(CopyGetters, Getters)]
pub struct Hardware {
  /// Device name
  #[getset(get = "pub")]
  name: String,
  /// Device address
  #[getset(get = "pub")]
  address: String,
  /// Communication endpoints
  #[getset(get = "pub")]
  endpoints: Vec<Endpoint>,
  /// Minimum time between two packets being sent to the device. Used to deal with congestion across
  /// protocols like Bluetooth LE, which have guaranteed delivery but can be overloaded due to
  /// connection intervals.
  #[getset(get_copy = "pub")]
  message_gap: Option<Duration>,
  ///  Internal implementation details
  internal_impl: Box<dyn HardwareInternal>,
  /// Requires a keepalive signal to be sent by the Server Device class
  #[getset(get_copy = "pub")]
  requires_keepalive: bool,
  last_write_time: Arc<RwLock<Instant>>,
}

impl Hardware {
  pub fn new(
    name: &str,
    address: &str,
    endpoints: &[Endpoint],
    message_gap: &Option<Duration>,
    requires_keepalive: bool,
    internal_impl: Box<dyn HardwareInternal>,
  ) -> Self {
    Self {
      name: name.to_owned(),
      address: address.to_owned(),
      endpoints: endpoints.into(),
      message_gap: message_gap.clone(),
      internal_impl,
      requires_keepalive,
      last_write_time: Arc::new(RwLock::new(Instant::now())),
    }
  }

  pub async fn time_since_last_write(&self) -> Duration {
    Instant::now().duration_since(*self.last_write_time.read().await)
  }

  /// Returns a receiver for any events the device may emit.
  ///
  /// This uses a broadcast channel and can be called multiple times to create multiple streams if
  /// needed.
  pub fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.internal_impl.event_stream()
  }

  /// Disconnect from the device (if it is connected)
  pub fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    self.internal_impl.disconnect()
  }

  pub fn parse_message(
    &self,
    command: &HardwareCommand,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    match command {
      HardwareCommand::Write(cmd) => self.write_value(cmd),
      HardwareCommand::Subscribe(cmd) => self.subscribe(cmd),
      HardwareCommand::Unsubscribe(cmd) => self.unsubscribe(cmd),
    }
  }

  /// Read a value from the device
  pub fn read_value(
    &self,
    msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    self.internal_impl.read_value(msg)
  }

  /// Write a value to the device
  pub fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let write_fut = self.internal_impl.write_value(msg);
    if self.requires_keepalive {
      let last_write_time = self.last_write_time.clone();
      async move {
        *last_write_time.write().await = Instant::now();
        write_fut.await
      }
      .boxed()
    } else {
      write_fut
    }
  }

  /// Subscribe to a device endpoint, if it exists
  pub fn subscribe(
    &self,
    msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    self.internal_impl.subscribe(msg)
  }

  /// Unsubscribe from a device endpoint, if it exists
  pub fn unsubscribe(
    &self,
    msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
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
  /// Disconnect from the device (if it is connected)
  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>>;
  /// Returns a receiver for any events the device may emit.
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent>;
  /// Read a value from the device
  fn read_value(
    &self,
    msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>>;
  /// Write a value to the device
  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>>;
  /// Subscribe to a device endpoint, if it exists
  fn subscribe(
    &self,
    msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>>;
  /// Unsubscribe from a device endpoint, if it exists
  fn unsubscribe(
    &self,
    msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>>;
}

#[async_trait]
pub trait HardwareConnector: Sync + Send + Debug {
  /// Return the hardware identifier for the device. Depends on the communication bus type, so may
  /// be a bluetooth name, serial port name, etc...
  fn specifier(&self) -> ProtocolCommunicationSpecifier;
  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError>;
}

#[async_trait]
pub trait HardwareSpecializer: Sync + Send {
  /// Try to initialize a device.
  ///
  /// Given a
  /// [ProtocolDeviceConfiguration](crate::server::device::configuration::ProtocolDeviceConfiguration)
  /// which will contain information about what a protocol needs to communicate with a device, try
  /// to identify all required endpoints on the hardware.
  async fn specialize(
    &mut self,
    protocol: &[ProtocolCommunicationSpecifier],
  ) -> Result<Hardware, ButtplugDeviceError>;
}

/// Used in cases where there's nothing to specialize for the protocol.
pub struct GenericHardwareSpecializer {
  hardware: Option<Hardware>,
}

impl GenericHardwareSpecializer {
  pub fn new(hardware: Hardware) -> Self {
    Self {
      hardware: Some(hardware),
    }
  }
}

#[async_trait]
impl HardwareSpecializer for GenericHardwareSpecializer {
  async fn specialize(
    &mut self,
    _: &[ProtocolCommunicationSpecifier],
  ) -> Result<Hardware, ButtplugDeviceError> {
    Ok(self.hardware.take().expect("This should only be run once"))
  }
}
