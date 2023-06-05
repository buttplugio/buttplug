// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Management of protocol and device hardware configurations
//!
//! Buttplug can handle device communication over several different mediums, including bluetooth,
//! usb, serial, various network protocols, and others. The library also provides multiple protocols
//! to communicate with this hardware. All of this information is stored in the
//! [DeviceConfigurationManager] (aka the DCM), a structure that is built whenever a [buttplug
//! server](crate::server::ButtplugServer) instance is created, and which is immutable for the life
//! of the server instance.
//!
//! The [DeviceConfigurationManager]'s main job is to take a newly discovered piece of hardware and
//! figure out if the library supports that hardware. To that end, the [DeviceConfigurationManager]
//! contains all of the APIs needed to load protocol configurations into the system, as well as
//! match newly discovered devices to protocols.
//!
//! ## Device Identification
//!
//! Once devices are connected, they are identified via the following properties:
//!
//! - Their communication bus address (BLE address, serial port name, etc... For devices that
//!   connect via network protocols, this may be a generated value, but should be unique.)
//! - Their protocol name
//! - Their protocol identifier
//!
//! These values are held in [ProtocolDeviceIdentifier] instances, and used around the codebase to
//! identify a device. This identifier is used so that if a device somehow shares addresses with
//! another device but identifies under a different protocol, they will still be seen as separate
//! devices.
//!
//! As an example, let's say we have a Lovense Hush. The protocol will be "lovense" (which is
//! configuration string version of the [Lovense Protocol](crate::device::protocol::lovense) name),
//! its identifier will be "Z" (the identification letter for Hush in Lovense's proprietary
//! protocol), and the address will be something like "AA:BB:CC:DD:EE:FF", which is the BLE address
//! of the device on platforms that provide BLE addresses. Using these 3 values means that, even if
//! for some reason the BLE address stays the same, if a device identifies differently (say, as a
//! Domi instead of a Hush), we won't try to reuse the same configuration.
//!
//! **NOTE THAT DEVICE IDENTIFIERS MAY NOT BE PORTABLE ACROSS PLATFORMS.** While these are used as
//! internal identifers as well as keys for user configurations, they may not work if used between,
//! say, Windows BLE and WebBluetooth, which provide different addressing schemes for devices.
//!
//! ## Device Configurations versus User Configurations
//!
//! Device Configurations are provided by the core Buttplug Team, and the configuration of all
//! currently supported devices is both compiled into the library as well as distributed as external
//! files (see the Device Configuration Files section below). However, users may want to set certain
//! per-device configurations, in which case, User Configurations can be used.
//!
//! User configurations include:
//!
//! - Device Allow/Deny Lists: library will either only connect to certain devices, or never connect
//!   to them, respectively.
//! - Reserved indexes: allows the same device to show up to clients on the same device index every
//!   time it connects
//! - Device configuration extensions: If a new device from a brand comes out and has not been added
//!   to the main Device Configuration file, or else a user creates their own DIY device that uses
//!   another protocol (hence it will never be in the main Device Configuration file as there may
//!   only be one of the device, period), a user can add an extension to an established protocol to
//!   provide new identifier information.
//! - User configured message attributes: limits that can be set for certain messages a device
//!   takes. For instance, setting an upper limit on the vibration speed of a vibrator so it will
//!   only go to 80% instead of 100%.
//!
//! User configurations can be added to the [DeviceConfigurationManager].
//!
//! ## Device Configuration Files
//!
//! While all device configuration can be created and handled via API calls, the library supports
//! 100s of devices, meaning doing this in code would be rather unwieldy, and any new device
//! additions would require library version revs. To allow for adding and updating configurations
//! possibly without the need for library updates, we externalize this configuration to JSON files.
//!
//! Similarly, GUIs and other utilities have been created to facilitate creation of User
//! Configurations, and these are also stored to files and loadable by the library.
//!
//! These files are handled in the [Device Configuration File Module in the Utils portion of the
//! library](crate::util::device_configuration). More information on the file format and loading
//! strategies can be found there.
//!
//! ## Architecture
//!
//! The [DeviceConfigurationManager] consists of a tree of types and usage flow that may be a bit
//! confusing, so we'll outline and summarize them here.
//!
//! At the top level is the [DeviceConfigurationManager] itself. It contains 4 different pieces of
//! information:
//!
//! - Protocol device specifiers and attributes
//! - Factory/Builder instances for [ButtplugProtocols](crate::device::protocol::ButtplugProtocol)
//! - Whether or not Raw Messages are allowed
//! - User configuration information (allow/deny lists, per-device protocol attributes, etc...)
//!
//! The [DeviceConfigurationManager] is created when a ButtplugServer comes up, and which time
//! protocols and user configurations can be added. After this, it is queried any time a new device
//! is found, to see whether a registered protocol is usable with that device.
//!
//! ### Adding Protocols
//!
//! Adding protocols to the DCM happens via the add_protocol_factory and remove_protocol_factory
//! methods.
//!
//! ### Protocol Device Specifiers
//!
//! In order to know if a discovered device can be used by Buttplug, it needs to be checked for
//! identifying information. The library use "specifiers" (like [BluetoothLESpecifier],
//! [USBSpecifier], etc...) for this. Specifiers contain device identification and connection
//! information, and we compare groups of specifiers in protocol configurations (as part of the
//! [ProtocolDeviceConfiguration] instance) with a specifier built from discovered devices to see if
//! there are any matches.
//!
//! For instance, we know the Bluetooth LE information for WeVibe toys, all of which is stored with
//! the WeVibe protocol configuration. The WeVibe protocol configuration has a Bluetooth LE
//! specifier with all of that information. When someone has a, say, WeVibe Ditto, they can turn it
//! on and put it into bluetooth discovery mode. If Buttplug is scanning for devices, we'll see the
//! Ditto, via its corresponding Bluetooth advertisement. Data from this advertisement can be turned
//! into a Bluetooth LE specifier. We can then match the specifier made from the advertisement
//! against all the protocol specifiers in the system, and find that this device will work with the
//! WeVibe protocol, at which point we'll move to the next step, protocol building.
//!
//! ### Protocol Building
//!
//! If a discovered device matches one or more protocol specifiers, a connection attempt begins,
//! where each matched protocol is given a chance to see if it can identify and communicate with the
//! device. If a protocol and device are matched, and connection is successful the initialized
//! protocol instance is returned, and becomes part of the
//! [ButtplugDevice](crate::device::ButtplugDevice) instance used by the
//! [ButtplugServer](crate::server::ButtplugServer).
//!
//! ### Raw Messages
//!
//! ### User Configurations
//!

mod server_device_message_attributes;
pub mod specifier;
pub use specifier::*;

pub use server_device_message_attributes::{
  ServerDeviceMessageAttributes,
  ServerDeviceMessageAttributesBuilder,
  ServerGenericDeviceMessageAttributes,
};

use super::protocol::{get_default_protocol_map, ProtocolIdentifierFactory, ProtocolSpecializer};
use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ButtplugDeviceMessageType, Endpoint},
  },
  server::device::ServerDeviceIdentifier,
};
use dashmap::DashMap;
use derivative::Derivative;
use getset::{Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};
use std::{
  collections::HashMap,
  sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
  },
};

/// Denotes what set of protocols attributes should be used: Default (generic) or device class
/// specific.
#[derive(Debug, Clone, Eq, Serialize, Deserialize, Derivative)]
#[derivative(Hash, PartialEq)]
pub enum ProtocolAttributesType {
  /// Default for all devices supported by a protocol
  Default,
  /// Device class specific identification, with a string specific to the protocol.
  Identifier(String),
}

/// A version of [ServerDeviceIdentifier] used for protocol lookup and matching.
///
/// This mirrors [ServerDeviceIdentifier], except that address is optional, as we will have protocol
/// attributes that pertain to sets of hardware as well as user configs, which only deal with a
/// single piece of hardware.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProtocolAttributesIdentifier {
  protocol: String,
  attributes_identifier: ProtocolAttributesType,
  address: Option<String>,
}

impl ProtocolAttributesIdentifier {
  pub fn new(
    protocol: &str,
    attributes_identifier: &ProtocolAttributesType,
    address: &Option<String>,
  ) -> Self {
    Self {
      protocol: protocol.to_owned(),
      attributes_identifier: attributes_identifier.clone(),
      address: address.clone(),
    }
  }
}

impl From<&ServerDeviceIdentifier> for ProtocolAttributesIdentifier {
  fn from(other: &ServerDeviceIdentifier) -> Self {
    Self {
      protocol: other.protocol().clone(),
      attributes_identifier: other.attributes_identifier().clone(),
      address: Some(other.address().clone()),
    }
  }
}

impl PartialEq<ServerDeviceIdentifier> for ProtocolAttributesIdentifier {
  fn eq(&self, other: &ServerDeviceIdentifier) -> bool {
    self.protocol == *other.protocol()
      && self.attributes_identifier == *other.attributes_identifier()
      && self.address == Some(other.address().clone())
  }
}

/// Device attribute storage and handling
///
/// ProtocolDeviceAttributes represent information about a device in relation to its protocol. This
/// includes the device name, its identifier (assuming it has one), its user created display name
/// (if it has one), and its message attributes.
///
/// Device attributes can exist in 3 different forms for a protocol, as denoted by the
/// [ProtocolAttributesIdentifier].
///
/// - Default: The basis for all message attributes for a protocol. Used when a protocol supports
///   many different devices, all with at least one or more similar features. For instances, we can
///   assume all Lovense devices have a single vibrator with a common power level count, so the
///   Default identifier instance of the ProtocolDeviceAttributes for Lovense will have a
///   message_attributes with VibrateCmd (assuming 1 vibration motor, as all Lovense devices have at
///   least one motor) available.
/// - Identifier: Specifies a specific device for a protocol, which may have its own attributes.
///   Continuing with the Lovense Example, we know a Edge will have 2 motors. We can set the
///   specific Identifier version of the ProtocolDeviceAttributes to have a VibrateCmd
///   message_attributes entry which will override the Default identifier version.
/// - User Configuration: Users may set configurations specific to their setup, like reducing the
///   maximum power available on a device to a certain level. User configurations override the
///   previous Identifier and Default configurations.
///
///  This type of tree/list encoding preserves the structure of configuration, which allows for
///  easier debugging, as well as the ability to serialize the structure back down to files.
#[derive(Debug, Clone, Getters, Setters, MutGetters)]
pub struct ProtocolDeviceAttributes {
  /// Identifies which type of attributes this instance represents for a protocol (Protocol default or device specific)
  identifier: ProtocolAttributesType,
  /// Parent of this device attributes instance. If any attribute is missing from this instance,
  /// we'll fall back to the parent to try and resolve it.
  parent: Option<Arc<ProtocolDeviceAttributes>>,
  /// Given name of the device this instance represents.
  name: Option<String>,
  /// User configured name of the device this instance represents, assuming one exists.
  display_name: Option<String>,
  /// Message attributes for this device instance.
  pub(super) message_attributes: ServerDeviceMessageAttributes,
}

impl ProtocolDeviceAttributes {
  /// Create a new instance
  pub fn new(
    identifier: ProtocolAttributesType,
    name: Option<String>,
    display_name: Option<String>,
    message_attributes: ServerDeviceMessageAttributes,
    parent: Option<Arc<ProtocolDeviceAttributes>>,
  ) -> Self {
    Self {
      identifier,
      name,
      display_name,
      message_attributes,
      parent,
    }
  }

  /// Create a new instance from an already created instance, compressing any call to parent nodes.
  ///
  /// We only need to preserve the tree encoding inside of the DeviceConfigurationManager. Once a
  /// attributes struct is handed out to the world, it is considered static, so we can provide a
  /// flattened representation.
  pub fn flatten(&self) -> Self {
    Self {
      identifier: self.identifier().clone(),
      parent: None,
      name: Some(self.name().to_owned()),
      display_name: self.display_name(),
      message_attributes: self.message_attributes(),
    }
  }

  /// Create a copy of an instance, but with a new parent.
  pub fn new_with_parent(&self, parent: Arc<ProtocolDeviceAttributes>) -> Self {
    Self {
      parent: Some(parent),
      ..self.clone()
    }
  }

  /// Return the protocol identifier for this instance
  pub fn identifier(&self) -> &ProtocolAttributesType {
    &self.identifier
  }

  /// Return the device name for this instance, or "Unknown Buttplug Device" if no name exists.
  pub fn name(&self) -> &str {
    if let Some(name) = &self.name {
      name
    } else if let Some(parent) = &self.parent {
      parent.name()
    } else {
      "Unknown Buttplug Device"
    }
  }

  /// Return the user configured display name for this instance, assuming one exists.
  pub fn display_name(&self) -> Option<String> {
    if let Some(name) = &self.display_name {
      Some(name.clone())
    } else if let Some(parent) = &self.parent {
      parent.display_name()
    } else {
      None
    }
  }

  /// Check to make sure the message attributes of an instance are valid.
  fn is_valid(&self) -> Result<(), ButtplugDeviceError> {
    if let Some(attrs) = self.message_attributes.scalar_cmd() {
      for attr in attrs {
        attr.is_valid(&ButtplugDeviceMessageType::ScalarCmd)?;
      }
    }
    if let Some(attrs) = self.message_attributes.rotate_cmd() {
      for attr in attrs {
        attr.is_valid(&ButtplugDeviceMessageType::RotateCmd)?;
      }
    }
    if let Some(attrs) = self.message_attributes.linear_cmd() {
      for attr in attrs {
        attr.is_valid(&ButtplugDeviceMessageType::LinearCmd)?;
      }
    }
    Ok(())
  }

  /// Check if a type of device message is supported by this instance.
  pub fn allows_message(&self, message_type: &ButtplugDeviceMessageType) -> bool {
    self.message_attributes.message_allowed(message_type)
  }

  /// Retreive a map of all message attributes for this instance.
  pub fn message_attributes(&self) -> ServerDeviceMessageAttributes {
    if let Some(parent) = &self.parent {
      parent.message_attributes().merge(&self.message_attributes)
    } else {
      self.message_attributes.clone()
    }
  }

  /// Add raw message support to the attributes of this instance. Requires a list of all endpoints a
  /// device supports.
  pub fn add_raw_messages(&mut self, endpoints: &[Endpoint]) {
    self.message_attributes.add_raw_messages(endpoints);
  }
}

#[derive(Default, Clone)]
pub struct DeviceConfigurationManagerBuilder {
  skip_default_protocols: bool,
  allow_raw_messages: bool,
  communication_specifiers: HashMap<String, Vec<ProtocolCommunicationSpecifier>>,
  protocol_attributes: HashMap<ProtocolAttributesIdentifier, ProtocolDeviceAttributes>,
  /// Map of protocol names to their respective protocol instance factories
  protocols: Vec<(String, Arc<dyn ProtocolIdentifierFactory>)>,
  /// Addresses of devices that we will only connect to, if this list is not empty. As these are
  /// checked before we actually connect to a device, they're the string serialized version of the
  /// address, versus using a [ServerDeviceIdentifier].
  allowed_addresses: Vec<String>,
  /// Address of devices we never want to connect to. As these are checked before we actually
  /// connect to a device, they're the string serialized version of the address, versus using a
  /// [ServerDeviceIdentifier].
  denied_addresses: Vec<String>,
  reserved_indexes: Vec<(ServerDeviceIdentifier, u32)>,
}

impl DeviceConfigurationManagerBuilder {
  pub fn merge(&mut self, other: &DeviceConfigurationManagerBuilder) -> &mut Self {
    self.skip_default_protocols = self.skip_default_protocols || other.skip_default_protocols;
    self.allow_raw_messages = self.allow_raw_messages || other.allow_raw_messages;
    self.communication_specifiers.extend(
      other
        .communication_specifiers
        .iter()
        .map(|(k, v)| (k.clone(), v.clone())),
    );
    self.protocol_attributes.extend(
      other
        .protocol_attributes
        .iter()
        .map(|(k, v)| (k.clone(), v.clone())),
    );
    self
      .protocols
      .extend(other.protocols.iter().map(|v| (v.clone())));
    self
      .allowed_addresses
      .extend(other.allowed_addresses.iter().map(|v| (v.clone())));
    self
      .denied_addresses
      .extend(other.denied_addresses.iter().map(|v| (v.clone())));
    self
      .reserved_indexes
      .extend(other.reserved_indexes.iter().map(|v| (v.clone())));
    self
  }

  pub fn communication_specifier(
    &mut self,
    protocol_name: &str,
    specifier: ProtocolCommunicationSpecifier,
  ) -> &mut Self {
    self
      .communication_specifiers
      .entry(protocol_name.to_owned())
      .or_default()
      .push(specifier);
    self
  }

  pub fn protocol_attributes(
    &mut self,
    identifier: ProtocolAttributesIdentifier,
    attributes: ProtocolDeviceAttributes,
  ) -> &mut Self {
    self.protocol_attributes.insert(identifier, attributes);
    self
  }

  /// Add a protocol instance factory for a [ButtplugProtocol]
  pub fn protocol_factory<T>(&mut self, factory: T) -> &mut Self
  where
    T: ProtocolIdentifierFactory + 'static,
  {
    self
      .protocols
      .push((factory.identifier().to_owned(), Arc::new(factory)));
    self
  }

  pub fn skip_default_protocols(&mut self) -> &mut Self {
    self.skip_default_protocols = true;
    self
  }

  pub fn allow_raw_messages(&mut self) -> &mut Self {
    self.allow_raw_messages = true;
    self
  }

  pub fn allowed_address(&mut self, address: &str) -> &mut Self {
    self.allowed_addresses.push(address.to_owned());
    self
  }

  pub fn denied_address(&mut self, address: &str) -> &mut Self {
    self.denied_addresses.push(address.to_owned());
    self
  }

  pub fn reserved_index(&mut self, identifier: &ServerDeviceIdentifier, index: u32) -> &mut Self {
    self.reserved_indexes.push((identifier.clone(), index));
    self
  }

  pub fn finish(&mut self) -> Result<DeviceConfigurationManager, ButtplugDeviceError> {
    // Map of protocol names to their respective protocol instance factories
    let mut protocol_map = if !self.skip_default_protocols {
      get_default_protocol_map()
    } else {
      HashMap::new()
    };

    for (name, protocol) in &self.protocols {
      if protocol_map.contains_key(name) {
        // TODO Fill in error
      }
      protocol_map.insert(name.clone(), protocol.clone());
    }

    // Build and validate the protocol attributes tree.
    let mut attribute_tree_map = HashMap::new();

    // Add all the defaults first, they won't have parent attributes.
    for (ident, attr) in self.protocol_attributes.iter().filter(|(ident, _)| {
      ident.attributes_identifier == ProtocolAttributesType::Default && ident.address.is_none()
    }) {
      // If we don't have a protocol loaded for this configuration block, just drop it. We can't do
      // anything with it anyways.
      if !protocol_map.contains_key(&ident.protocol) {
        continue;
      }
      attribute_tree_map.insert(ident.clone(), Arc::new(attr.clone()));
    }

    // Then add in everything that has a identifier but not an address, and possibly set their parents.
    for (ident, attr) in self.protocol_attributes.iter().filter(|(ident, _)| {
      matches!(
        ident.attributes_identifier,
        ProtocolAttributesType::Identifier(_)
      ) && ident.address.is_none()
    }) {
      // If we don't have a protocol loaded for this configuration block, just drop it. We can't do
      // anything with it anyways.
      if !protocol_map.contains_key(&ident.protocol) {
        continue;
      }
      if let Some(parent) = attribute_tree_map.get(&ProtocolAttributesIdentifier {
        address: None,
        protocol: ident.protocol.clone(),
        attributes_identifier: ProtocolAttributesType::Default,
      }) {
        let attr_with_parent = attr.new_with_parent(parent.clone());
        attribute_tree_map.insert(ident.clone(), Arc::new(attr_with_parent));
      } else {
        attribute_tree_map.insert(ident.clone(), Arc::new(attr.clone()));
      }
    }

    // Finally, add in user configurations, which will have an address.
    for (ident, attr) in self
      .protocol_attributes
      .iter()
      .filter(|(ident, _)| ident.address.is_some())
    {
      // If we don't have a protocol loaded for this configuration block, just drop it. We can't do
      // anything with it anyways.
      if !protocol_map.contains_key(&ident.protocol) {
        continue;
      }

      // The protocol and attribute identifier of a user config will be its parent. If that doesn't exist, error.
      if let Some(parent) = attribute_tree_map.get(&ProtocolAttributesIdentifier {
        address: None,
        protocol: ident.protocol.clone(),
        attributes_identifier: ident.attributes_identifier.clone(),
      }) {
        let attr_with_parent = attr.new_with_parent(parent.clone());
        attribute_tree_map.insert(ident.clone(), Arc::new(attr_with_parent));
      } else if let Some(parent) = attribute_tree_map.get(&ProtocolAttributesIdentifier {
        address: None,
        protocol: ident.protocol.clone(),
        attributes_identifier: ProtocolAttributesType::Default,
      }) {
        // There are some cases where protocols will hand back identifiers even though we don't have
        // any in the config (i.e. new devices we haven't added specializations for yet). In that
        // case, fall back to the default.
        let attr_with_parent = attr.new_with_parent(parent.clone());
        attribute_tree_map.insert(ident.clone(), Arc::new(attr_with_parent));
      } else {
        return Err(ButtplugDeviceError::DeviceConfigurationError(format!("User configuration {:?} does not have a parent type, cannot create configuration. Please remove this user configuration, or make sure it has a parent.", ident)));
      }
    }

    // Align the implementation, communication specifier, and attribute maps so we only keep what we
    // can actually use.

    let reserved_indexes = DashMap::new();
    for (identifier, index) in &self.reserved_indexes {
      if reserved_indexes.contains_key(identifier) {
        // TODO Fill in error
      }
      if reserved_indexes.iter().any(|pair| *pair == *index) {
        // TODO Fill in error
      }
      reserved_indexes.insert(identifier.clone(), *index);
    }

    // Make sure it's all valid.
    for attrs in attribute_tree_map.values() {
      attrs.is_valid()?;
    }

    Ok(DeviceConfigurationManager {
      allow_raw_messages: self.allow_raw_messages,
      communication_specifiers: self.communication_specifiers.clone(),
      protocol_attributes: attribute_tree_map,
      protocol_map,
      allowed_addresses: self.allowed_addresses.clone(),
      denied_addresses: self.denied_addresses.clone(),
      reserved_indexes,
      current_index: AtomicU32::new(0),
    })
  }
}

/// Correlates information about protocols and which devices they support.
///
/// The [DeviceConfigurationManager] handles stores information about which device protocols the
/// library supports, as well as which devices can use those protocols. When a
/// [DeviceCommunicationManager](crate::server::device::communication_manager) finds a device during scanning,
/// device information is given to the [DeviceConfigurationManager] to decide whether Buttplug
/// should try to connect to and communicate with the device.
///
/// Assuming the device is supported by the library, the [DeviceConfigurationManager] also stores
/// information about what commands can be sent to the device (Vibrate, Rotate, etc...), and the
/// parameters for those commands (number of power levels, stroke distances, etc...).
pub struct DeviceConfigurationManager {
  /// If true, add raw message support to connected devices
  allow_raw_messages: bool,
  communication_specifiers: HashMap<String, Vec<ProtocolCommunicationSpecifier>>,
  protocol_attributes: HashMap<ProtocolAttributesIdentifier, Arc<ProtocolDeviceAttributes>>,
  /// Map of protocol names to their respective protocol instance factories
  protocol_map: HashMap<String, Arc<dyn ProtocolIdentifierFactory>>,
  allowed_addresses: Vec<String>,
  denied_addresses: Vec<String>,
  reserved_indexes: DashMap<ServerDeviceIdentifier, u32>,
  current_index: AtomicU32,
}

impl Default for DeviceConfigurationManager {
  /// Create a new instance with Raw Message support turned off
  fn default() -> Self {
    // Unwrap allowed here because we assume our built in device config will
    // always work. System won't pass tests or possibly even build otherwise.
    DeviceConfigurationManagerBuilder::default()
      .finish()
      .expect("Default creation of a DCM should always work.")
  }
}

impl DeviceConfigurationManager {
  pub fn address_allowed(&self, address: &str) -> bool {
    let address = address.to_owned();
    // Make sure the device isn't on the deny list
    if self.denied_addresses.contains(&address) {
      // If device is outright denied, deny
      info!(
        "Device {} denied by configuration, not connecting.",
        address
      );
      false
    } else if !self.allowed_addresses.is_empty() && !self.allowed_addresses.contains(&address) {
      // If device is not on allow list and allow list isn't empty, deny
      info!(
        "Device {} not on allow list and allow list not empty, not connecting.",
        address
      );
      false
    } else {
      true
    }
  }

  pub fn device_index(&self, identifier: &ServerDeviceIdentifier) -> u32 {
    // See if we have a reserved or reusable device index here.
    if let Some(id) = self.reserved_indexes.get(identifier) {
      *id
    } else {
      let mut current_index = self.current_index.load(Ordering::SeqCst);
      while self.reserved_indexes.iter().any(|x| *x == current_index) {
        current_index += 1;
      }
      let generated_device_index = current_index;
      current_index += 1;
      self.current_index.store(current_index, Ordering::SeqCst);
      self
        .reserved_indexes
        .insert(identifier.clone(), generated_device_index);
      generated_device_index
    }
  }

  /// Provides read-only access to the internal protocol/identifier map. Mainly
  /// used for WebBluetooth filter construction, but could also be handy for
  /// listing capabilities in UI, etc.
  pub fn protocol_device_configurations(
    &self,
  ) -> HashMap<String, Vec<ProtocolCommunicationSpecifier>> {
    self.communication_specifiers.clone()
  }

  pub fn protocol_specializers(
    &self,
    specifier: &ProtocolCommunicationSpecifier,
  ) -> Vec<ProtocolSpecializer> {
    debug!(
      "Looking for protocol that matches specifier: {:?}",
      specifier
    );
    let mut specializers = vec![];
    for (name, specifiers) in self.communication_specifiers.iter() {
      if specifiers.contains(specifier) {
        info!("Found protocol {:?} for specifier {:?}.", name, specifier);

        if !self.protocol_map.contains_key(name) {
          warn!(
            "No protocol implementation for {:?} found for specifier {:?}.",
            name, specifier
          );
          continue;
        }
        specializers.push(ProtocolSpecializer::new(
          specifiers.clone(),
          self
            .protocol_map
            .get(name)
            .expect("already checked existence")
            .create(),
        ));
      }
    }
    specializers
  }

  pub fn protocol_device_attributes(
    &self,
    identifier: &ServerDeviceIdentifier,
    raw_endpoints: &[Endpoint],
  ) -> Option<ProtocolDeviceAttributes> {
    let mut flat_attrs = if let Some(attrs) = self.protocol_attributes.get(&identifier.into()) {
      debug!("User device config found for {:?}", identifier);
      attrs.flatten()
    } else if let Some(attrs) = self.protocol_attributes.get(&ProtocolAttributesIdentifier {
      address: None,
      attributes_identifier: identifier.attributes_identifier().clone(),
      protocol: identifier.protocol().clone(),
    }) {
      debug!(
        "Protocol + Identifier device config found for {:?}",
        identifier
      );
      attrs.flatten()
    } else if let Some(attrs) = self.protocol_attributes.get(&ProtocolAttributesIdentifier {
      address: None,
      attributes_identifier: ProtocolAttributesType::Default,
      protocol: identifier.protocol().clone(),
    }) {
      debug!("Protocol device config found for {:?}", identifier);
      attrs.flatten()
    } else {
      return None;
    };

    if self.allow_raw_messages {
      flat_attrs.add_raw_messages(raw_endpoints);
    }

    Some(flat_attrs)
  }
}

#[cfg(test)]
mod test {
  use super::{
    server_device_message_attributes::{
      ServerDeviceMessageAttributesBuilder,
      ServerGenericDeviceMessageAttributes,
    },
    *,
  };
  use std::{
    collections::{HashMap, HashSet},
    ops::RangeInclusive,
  };

  fn create_unit_test_dcm(allow_raw_messages: bool) -> DeviceConfigurationManager {
    let mut builder = DeviceConfigurationManagerBuilder::default();
    if allow_raw_messages {
      builder.allow_raw_messages();
    }
    let specifiers = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new(
      HashSet::from(["LVS-*".to_owned(), "LovenseDummyTestName".to_owned()]),
      vec![],
      HashSet::new(),
      HashMap::new(),
    ));
    builder.communication_specifier("lovense", specifiers);
    builder.protocol_attributes(
      ProtocolAttributesIdentifier::new(
        "lovense",
        &ProtocolAttributesType::Identifier("P".to_owned()),
        &None,
      ),
      ProtocolDeviceAttributes::new(
        ProtocolAttributesType::Identifier("P".to_owned()),
        Some("Lovense Edge".to_owned()),
        None,
        ServerDeviceMessageAttributesBuilder::default()
          .scalar_cmd(&vec![
            ServerGenericDeviceMessageAttributes::new(
              "Edge Vibrator 1",
              &RangeInclusive::new(0, 20),
              crate::core::message::ActuatorType::Vibrate,
            ),
            ServerGenericDeviceMessageAttributes::new(
              "Edge Vibrator 2",
              &RangeInclusive::new(0, 20),
              crate::core::message::ActuatorType::Vibrate,
            ),
          ])
          .finish(),
        None,
      ),
    );
    builder.finish().unwrap()
  }

  #[test]
  fn test_config_equals() {
    let config = create_unit_test_dcm(false);
    let spec = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LovenseDummyTestName",
      &HashMap::new(),
      &[],
    ));
    assert!(!config.protocol_specializers(&spec).is_empty());
  }

  #[test]
  fn test_config_wildcard_equals() {
    let config = create_unit_test_dcm(false);
    let spec = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Whatever",
      &HashMap::new(),
      &[],
    ));
    assert!(!config.protocol_specializers(&spec).is_empty());
  }

  #[test]
  fn test_specific_device_config_creation() {
    let dcm = create_unit_test_dcm(false);
    let spec = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Whatever",
      &HashMap::new(),
      &[],
    ));
    assert!(!dcm.protocol_specializers(&spec).is_empty());
    let config = dcm
      .protocol_device_attributes(
        &ServerDeviceIdentifier::new(
          "Whatever",
          "lovense",
          &ProtocolAttributesType::Identifier("P".to_owned()),
        ),
        &[],
      )
      .expect("Should be found");
    // Make sure we got the right name
    assert_eq!(config.name(), "Lovense Edge");
    // Make sure we overwrote the default of 1
    assert_eq!(
      config
        .message_attributes()
        .scalar_cmd()
        .as_ref()
        .expect("Test, assuming infallible")
        .get(0)
        .expect("Test, assuming infallible")
        .step_count(),
      20
    );
  }

  #[test]
  fn test_raw_device_config_creation() {
    let dcm = create_unit_test_dcm(true);
    let spec = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Whatever",
      &HashMap::new(),
      &[],
    ));
    assert!(!dcm.protocol_specializers(&spec).is_empty());
    let config = dcm
      .protocol_device_attributes(
        &ServerDeviceIdentifier::new(
          "Whatever",
          "lovense",
          &ProtocolAttributesType::Identifier("P".to_owned()),
        ),
        &[],
      )
      .expect("Should be found");
    // Make sure we got the right name
    assert_eq!(config.name(), "Lovense Edge");
    // Make sure we overwrote the default of 1
    assert!(config.message_attributes().raw_read_cmd().is_some());
    assert!(config.message_attributes().raw_write_cmd().is_some());
    assert!(config.message_attributes().raw_subscribe_cmd().is_some());
    assert!(config.message_attributes().raw_unsubscribe_cmd().is_some());
  }

  #[test]
  fn test_non_raw_device_config_creation() {
    let dcm = create_unit_test_dcm(false);
    let spec = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Whatever",
      &HashMap::new(),
      &[],
    ));
    assert!(!dcm.protocol_specializers(&spec).is_empty());
    let config = dcm
      .protocol_device_attributes(
        &ServerDeviceIdentifier::new(
          "Whatever",
          "lovense",
          &ProtocolAttributesType::Identifier("P".to_owned()),
        ),
        &[],
      )
      .expect("Should be found");
    // Make sure we got the right name
    assert_eq!(config.name(), "Lovense Edge");
    // Make sure we overwrote the default of 1
    assert!(config.message_attributes().raw_read_cmd().is_none());
    assert!(config.message_attributes().raw_write_cmd().is_none());
    assert!(config.message_attributes().raw_subscribe_cmd().is_none());
    assert!(config.message_attributes().raw_unsubscribe_cmd().is_none());
  }

  /*
      #[test]
      fn test_user_config_loading() {
        // Assume we have a nobra's entry in the device config.
        let mut config = create_test_dcm(false);
        assert!(config.protocol_definitions().contains_key("nobra"));
        assert!(config
          .protocol_definitions()
          .get("nobra")
          .expect("Test, assuming infallible")
          .serial
          .as_ref()
          .is_some());
        assert_eq!(
          config
            .protocol_definitions()
            .get("nobra")
            .expect("Test, assuming infallible")
            .serial
            .as_ref()
            .expect("Test, assuming infallible")
            .len(),
          1
        );

        // Now try overriding it, make sure we still only have 1.
        config = create_test_dcm(false);
        let mut nobra_def = ProtocolDefinition::default();
        let mut serial_specifier = SerialSpecifier::default();
        serial_specifier.port = "COM1".to_owned();
        nobra_def.serial = Some(vec![serial_specifier]);
        config.add_protocol_definition("nobra", nobra_def);
        assert!(config.protocol_definitions().contains_key("nobra"));
        assert!(config
          .protocol_definitions()
          .get("nobra")
          .expect("Test, assuming infallible")
          .serial
          .as_ref()
          .is_some());
        assert_eq!(
          config
            .protocol_definitions()
            .get("nobra")
            .expect("Test, assuming infallible")
            .serial
            .as_ref()
            .expect("Test, assuming infallible")
            .len(),
          1
        );
        assert!(config
          .protocol_definitions()
          .get("nobra")
          .expect("Test, assuming infallible")
          .serial
          .as_ref()
          .expect("Test, assuming infallible")
          .iter()
          .any(|x| x.port == "COM1"));
      }
  */
  // TODO Test invalid config load (not json)

  // TODO Test calculation/change of Step Count via Step Range
}
