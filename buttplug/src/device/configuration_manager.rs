// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Management of support devices, including identifying information and configurations.
//! 
//! ## Device Configuration and Discovery in Buttplug
//! 
//! Buttplug can handle device communication over several different mediums, including bluetooth,
//! usb, serial, various network protocols, and other means. The library can also identify which
//! protocol each device needs to use for command and control. All of this information is stored in
//! the [DeviceConfigurationManager], a structure that is built whenever a [buttplug
//! server](crate::server::ButtplugServer) instance is created, and which is immutable for the life
//! of the server instance.
//! 
//! The [DeviceConfigurationManager] contains all of the APIs needed to load protocol configurations
//! into the system, as well as match newly discovered devices to protocols. Protocols come with
//! "specifiers" (like [BluetoothLESpecifier], [USBSpecifier], etc...) which contain device
//! identification and connection information. If a discovered device matches one or more protocol
//! specifiers, a connection attempt begins, where each matched protocol is given a chance to see if
//! it can identify and communicate with the device. If a protocol and device are matched, and
//! connection is successful the initialized protocol instance is returned, and becomes part of the
//! [ButtplugDevice](crate::device::ButtplugDevice) instance used by the
//! [ButtplugServer](crate::server::ButtplugServer).
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

use super::protocol::{
  add_to_protocol_map, get_default_protocol_map, ButtplugProtocol, ButtplugProtocolFactory,
};
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::{ButtplugDeviceMessageType, DeviceMessageAttributes, DeviceMessageAttributesBuilder, DeviceMessageAttributesMap},
  },
  device::{DeviceImpl, Endpoint},
};
use dashmap::DashMap;
use getset::{Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};
use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
};
use uuid::Uuid;

// Note: There's a ton of extra structs in here just to deserialize the json
// file. Just leave them and build extras (for instance,
// DeviceProtocolConfiguration) if needed elsewhere in the codebase. It's not
// gonna hurt anything and making a ton of serde attributes is just going to get
// confusing (see the messages impl).

/// Specifier for Bluetooth LE Devices
/// 
/// Used by protocols for identifying bluetooth devices via their advertisements, as well as
/// defining the services and characteristics they are expected to have.
#[derive(Serialize, Deserialize, Debug, Clone, Getters, MutGetters, Setters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct BluetoothLESpecifier {
  /// Set of expected advertised names for this device.
  names: HashSet<String>,
  /// Set of expected advertised services for this device.
  #[serde(default, rename = "advertised-services")]
  advertised_services: HashSet<Uuid>,
  /// Services we expect the device may have. More services may be listed in a specifier than any
  /// one device may have, but we expect at least one to be matched by a device in order to consider
  /// the device part of the protocol that has this specifier.
  services: HashMap<Uuid, HashMap<Endpoint, Uuid>>,
}

impl PartialEq for BluetoothLESpecifier {
  fn eq(&self, other: &Self) -> bool {
    // If names or advertised services are found, use those automatically.
    if self.names.intersection(&other.names).count() > 0 {
      return true;
    }
    if self
      .advertised_services
      .intersection(&other.advertised_services)
      .count()
      > 0
    {
      return true;
    }
    // Otherwise, try wildcarded names.
    for name in &self.names {
      for other_name in &other.names {
        let compare_name: &String;
        let mut wildcard: String;
        if name.ends_with('*') {
          wildcard = name.clone();
          compare_name = other_name;
        } else if other_name.ends_with('*') {
          wildcard = other_name.clone();
          compare_name = name;
        } else {
          continue;
        }
        // Remove asterisk from the end of the wildcard
        wildcard.pop();
        if compare_name.starts_with(&wildcard) {
          return true;
        }
      }
    }
    false
  }
}

impl BluetoothLESpecifier {
  /// Creates a specifier from a BLE device advertisement.
  pub fn new_from_device(name: &str, advertised_services: &[Uuid]) -> BluetoothLESpecifier {
    let mut name_set = HashSet::new();
    name_set.insert(name.to_string());
    let service_set = HashSet::from_iter(advertised_services.iter().copied());
    BluetoothLESpecifier {
      names: name_set,
      advertised_services: service_set,
      services: HashMap::new(),
    }
  }

  /// Merge with another BLE specifier, used when loading user configs that extend a protocol
  /// definition.
  pub fn merge(&mut self, other: BluetoothLESpecifier) {
    // Add any new names.
    self.names = self.names.union(&other.names).cloned().collect();
    // Add new services, overwrite matching services.
    self.advertised_services = self.advertised_services.union(&other.advertised_services).cloned().collect();
    self.services.extend(other.services);
  }
}

/// Specifier for [Lovense Connect
/// Service](crate::server::comm_managers::lovense_connect_service) devices
/// 
/// Network based services, has no attributes because the [Lovense Connect
/// Service](crate::server::comm_managers::lovense_connect_service) device communication manager
/// handles all device discovery and identification itself.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LovenseConnectServiceSpecifier {
  // Needed for proper deserialization, but clippy will complain.
  #[allow(dead_code)]
  exists: bool,
}

impl Default for LovenseConnectServiceSpecifier {
  fn default() -> Self {
    Self { exists: true }
  }
}

impl PartialEq for LovenseConnectServiceSpecifier {
  fn eq(&self, _other: &Self) -> bool {
    true
  }
}

/// Specifier for [XInput](crate::server::comm_managers::xinput) devices
/// 
/// Network based services, has no attributes because the
/// [XInput](crate::server::comm_managers::xinput) device communication manager handles all device
/// discovery and identification itself.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct XInputSpecifier {
  // Needed for deserialziation but unused.
  #[allow(dead_code)]
  exists: bool,
}

impl Default for XInputSpecifier {
  fn default() -> Self {
    Self { exists: true }
  }
}

impl PartialEq for XInputSpecifier {
  fn eq(&self, _other: &Self) -> bool {
    true
  }
}

/// Specifier for HID (USB, Bluetooth) devices
/// 
/// Handles devices managed by the operating system's HID manager.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct HIDSpecifier {
  #[serde(rename = "vendor-id")]
  vendor_id: u16,
  #[serde(rename = "product-id")]
  product_id: u16,
}

/// Specifier for Serial devices
/// 
/// Handles serial port device identification (via port names) and configuration.
#[derive(Serialize, Deserialize, Debug, Clone, Default, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct SerialSpecifier {
  #[serde(rename = "baud-rate")]
  baud_rate: u32,
  #[serde(rename = "data-bits")]
  data_bits: u8,
  #[serde(rename = "stop-bits")]
  stop_bits: u8,
  parity: char,
  port: String,
}

impl SerialSpecifier {
  /// Given a serial port name (the only identifier we have for this type of device), create a
  /// specifier instance.
  pub fn new_from_name(port: &str) -> Self {
    SerialSpecifier {
      port: port.to_owned(),
      ..Default::default()
    }
  }
}

impl PartialEq for SerialSpecifier {
  fn eq(&self, other: &Self) -> bool {
    self.port == other.port
  }
}

/// Specifier for USB devices
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct USBSpecifier {
  #[serde(rename = "vendor-id")]
  vendor_id: u16,
  #[serde(rename = "product-id")]
  product_id: u16,
}

/// Specifier for Websocket Device Manager devices
///
/// The websocket device manager is a network based manager, so we have no info other than possibly
/// a device name that is provided as part of the connection handshake.
#[derive(Serialize, Deserialize, Debug, Clone, Default, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub")]
pub struct WebsocketSpecifier {
  pub names: HashSet<String>,
}

impl WebsocketSpecifier {
  pub fn merge(&mut self, other: WebsocketSpecifier) {
    // Just add the new identifier names
    self.names.extend(other.names);
  }
}

impl PartialEq for WebsocketSpecifier {
  fn eq(&self, other: &Self) -> bool {
    if self.names.intersection(&other.names).count() > 0 {
      return true;
    }
    false
  }
}

impl WebsocketSpecifier {
  pub fn new(name: &str) -> WebsocketSpecifier {
    let mut set = HashSet::new();
    set.insert(name.to_string());
    WebsocketSpecifier { names: set }
  }
}

/// Enum that covers all types of communication specifiers.
/// 
/// Allows generalization of specifiers to handle checking for equality. Used for testing newly discovered
/// devices against the list of known devices for a protocol.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ProtocolCommunicationSpecifier {
  BluetoothLE(BluetoothLESpecifier),
  HID(HIDSpecifier),
  USB(USBSpecifier),
  Serial(SerialSpecifier),
  XInput(XInputSpecifier),
  LovenseConnectService(LovenseConnectServiceSpecifier),
  Websocket(WebsocketSpecifier),
}

impl PartialEq for ProtocolCommunicationSpecifier {
  fn eq(&self, other: &ProtocolCommunicationSpecifier) -> bool {
    use ProtocolCommunicationSpecifier::*;
    match (self, other) {
      (USB(self_spec), USB(other_spec)) => self_spec == other_spec,
      (Serial(self_spec), Serial(other_spec)) => self_spec == other_spec,
      (BluetoothLE(self_spec), BluetoothLE(other_spec)) => self_spec == other_spec,
      (HID(self_spec), HID(other_spec)) => self_spec == other_spec,
      (XInput(self_spec), XInput(other_spec)) => self_spec == other_spec,
      (Websocket(self_spec), Websocket(other_spec)) => self_spec == other_spec,
      (LovenseConnectService(self_spec), LovenseConnectService(other_spec)) => {
        self_spec == other_spec
      }
      _ => false,
    }
  }
}

impl Eq for ProtocolCommunicationSpecifier {}

/// Identifying information for a connected devices
/// 
/// Contains the 3 fields needed to uniquely identify a device in the system.
#[derive(Debug, Eq, PartialEq, Hash, Clone, Getters, Setters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub(crate)", get_mut = "pub(crate)")]
pub struct ProtocolDeviceIdentifier {
  /// Address, as possibly serialized by whatever the managing library for the Device Communication Manager is.
  address: String,
  /// Name of the protocol used
  protocol: String,
  /// Internal identifier for the protocol used
  identifier: ProtocolAttributesIdentifier
}

impl ProtocolDeviceIdentifier {
  /// Creates a new instance
  pub fn new(address: &str, protocol: &str, identifier: &ProtocolAttributesIdentifier) -> Self {
    Self {
      address: address.to_owned(),
      protocol: protocol.to_owned(),
      identifier: identifier.clone()
    }
  }
}

#[derive(Debug, Clone, Getters, Setters, MutGetters)]
pub struct ProtocolDeviceAttributes {
  identifier: ProtocolAttributesIdentifier,
  parent: Option<Arc<ProtocolDeviceAttributes>>,
  name: Option<String>,
  display_name: Option<String>,
  pub(super) message_attributes: DeviceMessageAttributesMap,
}

impl ProtocolDeviceAttributes {
  pub fn new(
    identifier: ProtocolAttributesIdentifier,
    name: Option<String>,
    display_name: Option<String>,
    message_attributes: DeviceMessageAttributesMap,
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

  // We only need to preserve the tree encoding inside of the DeviceConfigurationManager. Once a
  // attributes struct is handed out to the world, it is considered static, so we can provide a
  // flattened representation.
  pub fn new_flattened(other: &ProtocolDeviceAttributes) -> Self {
    Self {
      identifier: other.identifier().clone(),
      parent: None,
      name: Some(other.name().to_owned()),
      display_name: other.display_name(),
      message_attributes: other.message_attributes_map(),
    }
  }

  pub fn new_with_parent(&self, parent: Arc<ProtocolDeviceAttributes>) -> Self {
    Self {
      parent: Some(parent),
      .. self.clone()
    }
  }

  fn is_valid(&self) -> Result<(), ButtplugError> { 
    for (message_type, message_attrs) in self.message_attributes_map() {
      message_attrs.check(&message_type).map_err(|err| {
        info!("Error in {}: {:?}", message_type, message_attrs);
        ButtplugError::from(err)
      })?;
    }
    Ok(())
  }
  
  pub fn identifier(&self) -> &ProtocolAttributesIdentifier {
    &self.identifier
  }

  pub fn name(&self) -> &str {
    if let Some(name) = &self.name {
      name
    } else if let Some(parent) = &self.parent {
      parent.name()
    } else {
      "Unknown Buttplug Device"
    }
  }

  pub fn display_name(&self) -> Option<String> {
    if let Some(name) = &self.display_name {
      Some(name.clone())
    } else if let Some(parent) = &self.parent {
      parent.display_name()
    } else {
      None
    }
  }

  pub fn allows_message(
    &self,
    message_type: &ButtplugDeviceMessageType,
  ) -> bool {
    self
      .message_attributes
      .contains_key(message_type)
  }

  pub fn message_attributes(
    &self,
    message_type: &ButtplugDeviceMessageType,
  ) -> Option<DeviceMessageAttributes> {
    if let Some(attributes) = self.message_attributes.get(message_type) {
      Some(attributes.clone())
    } else if let Some(parent) = &self.parent {
      parent.message_attributes(message_type)
    } else {
      None
    }
  }

  pub fn message_attributes_map(&self) -> DeviceMessageAttributesMap {
    if let Some(parent) = &self.parent {
      let mut map = parent.message_attributes_map();
      for (message, value) in &self.message_attributes {
        let attrs = map
          .get(message)
          .map(|base_attrs| base_attrs.merge(value))
          .or_else(|| Some(value.clone()))
          .expect("We filled in the device attributes either way.");
        // Overwrite anything that might already be in the map with our new attribute set.
        map.insert(*message, attrs);
      }
      map
    } else {
      self.message_attributes.clone()
    }
  }

  pub fn add_raw_messages(&mut self, endpoints: &[Endpoint]) {
    let endpoint_attributes = DeviceMessageAttributesBuilder::default()
      .endpoints(endpoints.to_owned())
      .build(&ButtplugDeviceMessageType::RawReadCmd)
      .expect("Nothing needs checking");

    self.message_attributes.insert(
      ButtplugDeviceMessageType::RawReadCmd,
      endpoint_attributes.clone(),
    );
    self.message_attributes.insert(
      ButtplugDeviceMessageType::RawWriteCmd,
      endpoint_attributes.clone(),
    );
    self.message_attributes.insert(
      ButtplugDeviceMessageType::RawSubscribeCmd,
      endpoint_attributes.clone(),
    );
    self.message_attributes.insert(
      ButtplugDeviceMessageType::RawUnsubscribeCmd,
      endpoint_attributes,
    );
  }
}

#[derive(Debug, Clone, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolAttributesIdentifier {
  // The default protocol attribute identifier
  Default,
  Identifier(String),
}

impl PartialEq for ProtocolAttributesIdentifier {
  fn eq(&self, other: &Self) -> bool {
    use ProtocolAttributesIdentifier::*;
    match (self, other) {
      (Default, Default) => true,
      (Identifier(ident1), Identifier(ident2)) => ident1 == ident2,
      _ => false,
    }
  }
}

#[derive(Debug, Clone, Getters, MutGetters, Default)]
pub struct ProtocolDeviceConfiguration {
  #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
  specifiers: Vec<ProtocolCommunicationSpecifier>,
  #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
  configurations: HashMap<ProtocolAttributesIdentifier, Arc<ProtocolDeviceAttributes>>,
}

impl ProtocolDeviceConfiguration {
  pub fn new(
    specifiers: Vec<ProtocolCommunicationSpecifier>,
    configurations: HashMap<ProtocolAttributesIdentifier, Arc<ProtocolDeviceAttributes>>,
  ) -> Self {
    Self {
      specifiers,
      configurations,
    }
  }

  pub fn is_valid(&self) -> Result<(), ButtplugError> {
    for (ident, attrs) in &self.configurations {
      attrs.is_valid().map_err(|e| ButtplugDeviceError::DeviceConfigurationError(format!("Error in {ident:?} configuration: {e}")))?;
    }
    Ok(())
  }

  pub fn device_attributes(
    &self,
    identifier: &ProtocolAttributesIdentifier,
  ) -> Option<&Arc<ProtocolDeviceAttributes>> {
    self.configurations.get(identifier)
  }
}

#[derive(Clone, Debug)]
pub struct ProtocolDeviceAttributesBuilder {
  protocol_identifier: String,
  allow_raw_messages: bool,
  device_configuration: ProtocolDeviceConfiguration,
  user_configs: Arc<DashMap<ProtocolDeviceIdentifier, ProtocolDeviceAttributes>>,
}

impl ProtocolDeviceAttributesBuilder {
  fn new(protocol_identifier: &str, allow_raw_messages: bool, device_configuration: ProtocolDeviceConfiguration, user_configs: Arc<DashMap<ProtocolDeviceIdentifier, ProtocolDeviceAttributes>>) -> Self {
    Self {
      protocol_identifier: protocol_identifier.to_owned(),
      allow_raw_messages,
      device_configuration,
      user_configs
    }
  }

  pub fn create_from_device_impl(
    &self,
    device_impl: &Arc<DeviceImpl>,
  ) -> Result<ProtocolDeviceAttributes, ButtplugError> {
    self.create(
      device_impl.address(),
      &ProtocolAttributesIdentifier::Identifier(device_impl.name().to_owned()),
      &device_impl.endpoints(),
    )
  }

  pub fn create(
    &self,
    address: &str,
    identifier: &ProtocolAttributesIdentifier,
    endpoints: &[Endpoint],
  ) -> Result<ProtocolDeviceAttributes, ButtplugError> {
    // Skip checking for address here, addresses, should only be in the user config map
    let device_attributes = self
      .device_configuration
      .device_attributes(identifier)
      .or_else(|| {
        self
          .device_configuration
          .device_attributes(&ProtocolAttributesIdentifier::Default)
      })
      .ok_or_else(|| ButtplugError::from(
        ButtplugDeviceError::DeviceConfigurationError(format!(
          "Configuration not found for device identifier '{:?}' Address '{:?}'",
          identifier, address
        )),
      ))?;

    let device_identifier = ProtocolDeviceIdentifier::new(address, &self.protocol_identifier, identifier);

    // In the case we have a user config that matches the address of our device, build a new
    // ProtocolDeviceAttributes leaf node using our current identifier as the parent. Then check if
    // the new attributes are valid, falling back if they aren't.
    let mut attributes = if let Some(user_config) = self.user_configs.get(&device_identifier) {
      let new_attributes = user_config.new_with_parent(device_attributes.clone());
      if new_attributes.is_valid().is_ok() {
        ProtocolDeviceAttributes::new_flattened(&new_attributes)
      } else {
        error!("Invalid device attributes found in user config, falling back to main config attributes");
        ProtocolDeviceAttributes::new_flattened(device_attributes)
      }
    } else {
      ProtocolDeviceAttributes::new_flattened(device_attributes)
    };

    // If we're allowing raw messages, tack those on beforehand also.
    if self.allow_raw_messages {
      attributes.add_raw_messages(endpoints);
    }

    // Everything needs to be able to stop.
    attributes
      .message_attributes
      .entry(ButtplugDeviceMessageType::StopDeviceCmd)
      .or_insert_with(DeviceMessageAttributes::default);

    Ok(attributes)
  }
}

#[derive(Clone, Debug)]
pub struct ProtocolInstanceFactory {
  allow_raw_messages: bool,
  protocol_factory: Arc<dyn ButtplugProtocolFactory>,
  user_device_configs: Arc<DashMap<ProtocolDeviceIdentifier, ProtocolDeviceAttributes>>,
  configuration: ProtocolDeviceConfiguration,
}

impl ProtocolBuilder {
  fn new(
    allow_raw_messages: bool,
    protocol_factory: Arc<dyn ButtplugProtocolFactory>,
    user_device_configs: Arc<DashMap<ProtocolDeviceIdentifier, ProtocolDeviceAttributes>>,
    configuration: ProtocolDeviceConfiguration,
  ) -> Self {
    Self {
      allow_raw_messages,
      protocol_factory,
      user_device_configs,
      configuration,
    }
  }

  pub async fn create(
    &self,
    device_impl: Arc<DeviceImpl>,
  ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
    let builder = ProtocolDeviceAttributesBuilder::new(
      self.protocol_factory.protocol_identifier(),
      self.allow_raw_messages, 
      self.configuration.clone(), 
      self.user_device_configs.clone()
    );
    self.protocol_factory.try_create(device_impl.clone(), builder).await
  }

  pub fn configuration(&self) -> &ProtocolDeviceConfiguration {
    &self.configuration
  }
}

pub struct DeviceConfigurationManager {
  allow_raw_messages: bool,
  protocol_device_configurations: Arc<DashMap<String, ProtocolDeviceConfiguration>>,
  protocol_map: Arc<DashMap<String, Arc<dyn ButtplugProtocolFactory>>>,
  user_device_configs: Arc<DashMap<ProtocolDeviceIdentifier, ProtocolDeviceAttributes>>
}

impl Default for DeviceConfigurationManager {
  fn default() -> Self {
    // Unwrap allowed here because we assume our built in device config will
    // always work. System won't pass tests or possibly even build otherwise.
    Self::new(false)
  }
}

impl DeviceConfigurationManager {
  pub fn new(allow_raw_messages: bool) -> Self {
    Self {
      allow_raw_messages,
      protocol_device_configurations: Arc::new(DashMap::new()),
      protocol_map: Arc::new(get_default_protocol_map()),
      user_device_configs: Arc::new(DashMap::new()),
    }
  }

  pub fn add_user_device_config(&self, protocol_identifier: &ProtocolDeviceIdentifier, protocol_attributes: &ProtocolDeviceAttributes) -> Result<(), ButtplugError> {
    self.user_device_configs.insert(protocol_identifier.clone(), protocol_attributes.clone());
    Ok(())
  }

  pub fn remove_user_device_config(&self, protocol_identifier: &ProtocolDeviceIdentifier) {
    self.user_device_configs.remove(protocol_identifier);
  }

  pub fn user_device_config(&self, protocol_identifier: &ProtocolDeviceIdentifier) -> Option<ProtocolDeviceAttributes> {
    self.user_device_configs.get(protocol_identifier).and_then(|p| Some(p.value().clone()))
  }

  pub fn add_protocol_device_configuration(
    &self,
    protocol_name: &str,
    protocol_definition: &ProtocolDeviceConfiguration,
  ) -> Result<(), ButtplugError> {
    protocol_definition.is_valid()?;

    self
      .protocol_device_configurations
      .insert(protocol_name.to_owned(), protocol_definition.clone());
    Ok(())
  }

  pub fn remove_protocol_device_configuration(&self, protocol_name: &str) {
    self.protocol_device_configurations.remove(protocol_name);
  }

  pub fn add_protocol_factory<T>(&self, factory: T) -> Result<(), ButtplugDeviceError>
  where
    T: ButtplugProtocolFactory + 'static
  {
    if !self.protocol_map.contains_key(factory.protocol_identifier()) {
      add_to_protocol_map(&self.protocol_map, factory);
      Ok(())
    } else {
      Err(ButtplugDeviceError::ProtocolAlreadyAdded(
        factory.protocol_identifier().to_owned(),
      ))
    }
  }

  pub fn remove_protocol_factory(&self, protocol_identifier: &str) -> Result<(), ButtplugDeviceError> {
    if self.protocol_map.contains_key(protocol_identifier) {
      self.protocol_map.remove(protocol_identifier);
      Ok(())
    } else {
      Err(ButtplugDeviceError::ProtocolNotImplemented(
        protocol_identifier.to_owned(),
      ))
    }
  }

  pub fn remove_all_protocol_factories(&self) {
    self.protocol_map.clear();
  }

  pub fn protocol_factory(&self, protocol_name: &str) -> Option<Arc<dyn ButtplugProtocolFactory>> {
    self
      .protocol_map
      .get(protocol_name)
      .and_then(|r| Some(r.value().clone()))
  }

  /// Provides read-only access to the internal protocol/identifier map. Mainly
  /// used for WebBluetooth filter construction, but could also be handy for
  /// listing capabilities in UI, etc.
  pub fn protocol_device_configurations(
    &self,
  ) -> Arc<DashMap<String, ProtocolDeviceConfiguration>> {
    self.protocol_device_configurations.clone()
  }

  pub fn protocol_instance_factory(&self, specifier: &ProtocolCommunicationSpecifier) -> Option<ProtocolInstanceFactory> {
    debug!(
      "Looking for protocol that matches specifier: {:?}",
      specifier
    );
    for config in self.protocol_device_configurations.iter() {
      if config.value().specifiers.contains(specifier) {
        info!(
          "Found protocol configuration {:?} for specifier {:?}.",
          config.key(),
          specifier
        );

        if !self.protocol_map.contains_key(config.key()) {
          warn!(
            "No protocol implementation for {:?} found for specifier {:?}.",
            config.key(),
            specifier
          );
          return None;
        }

        let protocol_factory = self
          .protocol_map
          .get(config.key())
          .map(|pair| pair.value().clone())?;

        return Some(ProtocolInstanceFactory::new(
          self.allow_raw_messages,
          protocol_factory,
          self.user_device_configs.clone(),
          config.value().clone(),
        ));
      }
    }
    debug!("No protocol found for specifier {:?}.", specifier);
    None
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use std::collections::{HashMap, HashSet};
  use crate::{
    core::messages::ButtplugDeviceMessageType, device::Endpoint
  };

  fn create_unit_test_dcm(allow_raw_messages: bool) -> DeviceConfigurationManager {
    let dcm = DeviceConfigurationManager::new(allow_raw_messages);
    let specifiers = vec![ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier {
      names: HashSet::from(["LVS-*".to_owned(), "LovenseDummyTestName".to_owned()]),
      services: HashMap::new(),
      advertised_services: HashSet::new()
    })];
    let mut attributes = HashMap::new();
    attributes.insert(ProtocolAttributesIdentifier::Identifier("P".to_owned()), Arc::new(ProtocolDeviceAttributes::new(ProtocolAttributesIdentifier::Identifier("P".to_owned()), Some("Lovense Edge".to_owned()), None, HashMap::new(), None)));
    let pdc = ProtocolDeviceConfiguration::new(specifiers, attributes);
    dcm.add_protocol_device_configuration("lovense", &pdc).unwrap();
    dcm
  }

  #[test]
  fn test_config_equals() {
    let config = create_unit_test_dcm(false);
    let launch =
      ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device("LovenseDummyTestName", &[]));
    assert!(config.protocol_instance_factory(&launch).is_some());
  }

  #[test]
  fn test_config_wildcard_equals() {
    let config = create_unit_test_dcm(false);
    let lovense = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Whatever",
      &[],
    ));
    assert!(config.protocol_instance_factory(&lovense).is_some());
  }

  #[test]
  #[ignore]
  fn test_specific_device_config_creation() {
    let config = create_unit_test_dcm(false);
    let lovense = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Whatever",
      &[],
    ));
    let builder = config
      .protocol_instance_factory(&lovense)
      .expect("Test, assuming infallible");
    let config = builder
      .configuration()
      .device_attributes(&ProtocolAttributesIdentifier::Identifier("P".to_owned()))
      .expect("Test, assuming infallible");
    // Make sure we got the right name
    assert_eq!(config.name(), "Lovense Edge");
    // Make sure we overwrote the default of 1
    assert_eq!(
      config
        .message_attributes(&ButtplugDeviceMessageType::VibrateCmd)
        .expect("Test, assuming infallible")
        .feature_count()
        .expect("Test, assuming infallible"),
      2
    );
  }

  #[test]
  fn test_raw_device_config_creation() {
    let config = create_unit_test_dcm(true);
    let lovense = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Whatever",
      &[],
    ));
    let builder = config
      .protocol_instance_factory(&lovense)
      .expect("Test, assuming infallible");
    let device_attr_builder = ProtocolDeviceAttributesBuilder::new("lovense", true, builder.configuration().clone(), Arc::new(DashMap::new()));
    let config = device_attr_builder
      .create("DoesNotMatter", &ProtocolAttributesIdentifier::Identifier("P".to_owned()), &vec![Endpoint::Tx, Endpoint::Rx])
      .expect("Test, assuming infallible");
    // Make sure we got the right name
    assert_eq!(config.name(), "Lovense Edge");
    // Make sure we overwrote the default of 1
    assert!(config.allows_message(&ButtplugDeviceMessageType::RawWriteCmd));
    assert!(config.allows_message(&ButtplugDeviceMessageType::RawReadCmd));
    assert!(config.allows_message(&ButtplugDeviceMessageType::RawSubscribeCmd));
    assert!(config.allows_message(&ButtplugDeviceMessageType::RawUnsubscribeCmd));
  }

  #[test]
  fn test_non_raw_device_config_creation() {
    let config = create_unit_test_dcm(false);
    let lovense = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Whatever",
      &[],
    ));
    let builder = config
      .protocol_instance_factory(&lovense)
      .expect("Test, assuming infallible");
      let device_attr_builder = ProtocolDeviceAttributesBuilder::new("lovense", false, builder.configuration().clone(), Arc::new(DashMap::new()));
      let config = device_attr_builder
        .create(&"DoesNotMatter", &ProtocolAttributesIdentifier::Identifier("P".to_owned()), &vec![Endpoint::Tx, Endpoint::Rx])
        .expect("Test, assuming infallible");
      // Make sure we got the right name
      assert_eq!(config.name(), "Lovense Edge");
    // Make sure we got the right name
    assert_eq!(config.name(), "Lovense Edge");
    // Make sure we overwrote the default of 1
    assert!(!config.allows_message(&ButtplugDeviceMessageType::RawWriteCmd));
    assert!(!config.allows_message(&ButtplugDeviceMessageType::RawReadCmd));
    assert!(!config.allows_message(&ButtplugDeviceMessageType::RawSubscribeCmd));
    assert!(!config.allows_message(&ButtplugDeviceMessageType::RawUnsubscribeCmd));
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
