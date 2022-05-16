// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use once_cell::sync::OnceCell;
use std::{
  fmt::{self, Debug},
  sync::Arc,
};

use crate::{
  core::{
    errors::ButtplugError,
    messages::{
      ButtplugDeviceCommandMessageUnion,
      DeviceMessageAttributesMap,
      Endpoint,
      RawSubscribeCmd,
    },
    ButtplugResultFuture,
  },
  server::{
    ButtplugServerResultFuture,
    device::{
      hardware::{Hardware, HardwareCreator, HardwareEvent},
      configuration::{ProtocolInstanceFactory, ProtocolAttributesIdentifier},
      protocol::ButtplugProtocol,
    },
  },
};
use getset::{Getters, Setters, MutGetters};
use serde::{Serialize, Deserialize};
use core::hash::{Hash, Hasher};
use tokio::sync::broadcast;

/// Identifying information for a connected devices
/// 
/// Contains the 3 fields needed to uniquely identify a device in the system.
#[derive(Debug, Eq, PartialEq, Hash, Clone, Getters, Setters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub(crate)", get_mut = "pub(crate)")]
pub struct ServerDeviceIdentifier {
  /// Address, as possibly serialized by whatever the managing library for the Device Communication Manager is.
  address: String,
  /// Name of the protocol used
  protocol: String,
  /// Internal identifier for the protocol used
  identifier: ProtocolAttributesIdentifier
}

impl ServerDeviceIdentifier {
  /// Creates a new instance
  pub fn new(address: &str, protocol: &str, identifier: &ProtocolAttributesIdentifier) -> Self {
    Self {
      address: address.to_owned(),
      protocol: protocol.to_owned(),
      identifier: identifier.clone()
    }
  }
}

/// Main internal device representation structure
/// 
/// A ButtplugDevice is made up of 2 components:
/// 
/// - A [Device Implementation](crate::device::Hardware), which handles hardware connection and
///   communication.
/// - A [Protocol](crate::device::protocol::ButtplugProtocol), which takes Buttplug Commands and
///   translated them into propreitary commands to send to a device.
/// 
/// When a ButtplugDevice instance is created, it can be assumed that it represents a device that is
/// connected and has been successfully initialized. The instance will manage communication of all
/// commands sent from a [client](crate::client::ButtplugClient) that pertain to this specific
/// hardware.
pub struct ServerDevice {
  /// Protocol instance for the device
  protocol: Box<dyn ButtplugProtocol>,
  /// Hardware implementation for the device
  device: Arc<Hardware>,
  /// Display name for the device
  display_name: OnceCell<String>,
  /// Unique identifier for the device
  device_identifier: ServerDeviceIdentifier
}

impl Debug for ServerDevice {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ButtplugDevice")
      .field("name", &self.name())
      .field("identifier", &self.device_identifier())
      .finish()
  }
}

impl Hash for ServerDevice {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.device_identifier().hash(state);
  }
}

impl Eq for ServerDevice {
}

impl PartialEq for ServerDevice {
  fn eq(&self, other: &Self) -> bool {
    self.device_identifier() == other.device_identifier()
  }
}

impl ServerDevice {
  /// Given a protocol and a device impl, create a new ButtplugDevice instance
  fn new(protocol: Box<dyn ButtplugProtocol>, device: Arc<Hardware>) -> Self {
    Self {
      device_identifier: ServerDeviceIdentifier::new(device.address(), protocol.protocol_identifier(), protocol.protocol_attributes_identifier()),
      protocol,
      device,
      display_name: OnceCell::new(),
    }
  }

  /// Returns the device identifier
  pub fn device_identifier(&self) -> &ServerDeviceIdentifier {
    &self.device_identifier
  }

  /// Returns the address of the device implementation
  pub fn hardware_address(&self) -> &str {
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
  /// [DeviceConfigurationManager](crate::server::device::configuration::DeviceConfigurationManager),
  /// try to find a protocol that has information matching this device. This may include name match,
  /// port matching, etc...
  /// 
  /// If a matching protocol is found, we then call
  /// [ButtplugHardwareCreator::try_create_hardware](crate::device::ButtplugHardwareCreator::try_create_hardware)
  /// with the related protocol information, in order to connect and initialize the device.
  /// 
  /// If all of that is successful, we return a ButtplugDevice that is ready to advertise to the
  /// client and use.
  pub async fn try_create_device(
    protocol_builder: ProtocolInstanceFactory,
    mut device_creator: Box<dyn HardwareCreator>,
  ) -> Result<Option<ServerDevice>, ButtplugError> {
    // TODO This seems needlessly complex, can we clean up how we pass the device builder and protocol factory around?
    
    // Now that we have both a possible device implementation and a configuration for that device,
    // try to initialize the implementation. This usually means trying to connect to whatever the
    // device is, finding endpoints, etc.
    let hardware = device_creator.try_create_hardware(protocol_builder.configuration().clone()).await?;
    info!(
      address = tracing::field::display(hardware.address()),
      "Found Buttplug Device {}",
      hardware.name()
    );

    // If we've made it this far, we now have a connected device implementation with endpoints set
    // up. We now need to run whatever protocol initialization might need to happen. We'll fetch a
    // protocol creator, pass the device implementation to it, then let it do whatever it needs. For
    // most protocols, this is a no-op. However, for devices like Lovense, some Kiiroo, etc, this
    // can get fairly complicated.
    let sharable_hardware = Arc::new(hardware);
    let protocol_impl =
      protocol_builder.create(sharable_hardware.clone()).await?;
    Ok(Some(ServerDevice::new(
      protocol_impl,
      sharable_hardware,
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
  ) -> ButtplugServerResultFuture {
    self.protocol.handle_command(self.device.clone(), message)
  }

  /// Retreive the event stream for the device.
  /// 
  /// This will include connections, disconnections, and notification events from subscribed
  /// endpoints.
  pub fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.device.event_stream()
  }
}
