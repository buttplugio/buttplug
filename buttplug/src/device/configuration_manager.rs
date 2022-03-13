// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2021 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Device specific identification and protocol implementations.

use super::protocol::{
  add_to_protocol_map, get_default_protocol_map, ButtplugProtocol, TryCreateProtocolFunc,
};
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::{ButtplugDeviceMessageType, DeviceMessageAttributes, DeviceMessageAttributesMap},
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

#[derive(Serialize, Deserialize, Debug, Clone, Getters, MutGetters, Setters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct BluetoothLESpecifier {
  names: HashSet<String>,
  #[serde(default, rename = "advertised-services")]
  advertised_services: HashSet<Uuid>,
  // Set of services that we may have gotten as part of the advertisement.
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

  pub fn merge(&mut self, other: BluetoothLESpecifier) {
    // Add any new names.
    self.names = self.names.union(&other.names).cloned().collect();
    // Add new services, overwrite matching services.
    self.advertised_services = self.advertised_services.union(&other.advertised_services).cloned().collect();
    self.services.extend(other.services);
  }
}

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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct HIDSpecifier {
  #[serde(rename = "vendor-id")]
  vendor_id: u16,
  #[serde(rename = "product-id")]
  product_id: u16,
}

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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct USBSpecifier {
  #[serde(rename = "vendor-id")]
  vendor_id: u16,
  #[serde(rename = "product-id")]
  product_id: u16,
}

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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ProtocolDeviceSpecifier {
  BluetoothLE(BluetoothLESpecifier),
  HID(HIDSpecifier),
  USB(USBSpecifier),
  Serial(SerialSpecifier),
  XInput(XInputSpecifier),
  LovenseConnectService(LovenseConnectServiceSpecifier),
  Websocket(WebsocketSpecifier),
}

impl ProtocolDeviceSpecifier {
  pub fn matches(&self, other: &ProtocolDeviceSpecifier) -> bool {
    use ProtocolDeviceSpecifier::*;
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

#[derive(Debug, Clone, Default, Getters, Setters, MutGetters)]
pub struct ProtocolDeviceAttributes {
  parent: Option<Arc<ProtocolDeviceAttributes>>,
  name: Option<String>,
  display_name: Option<String>,
  pub(super) message_attributes: DeviceMessageAttributesMap,
}

impl ProtocolDeviceAttributes {
  pub fn new(
    name: Option<String>,
    display_name: Option<String>,
    message_attributes: DeviceMessageAttributesMap,
    parent: Option<Arc<ProtocolDeviceAttributes>>,
  ) -> Self {
    Self {
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
      parent: None,
      name: Some(other.name().to_owned()),
      display_name: other.display_name(),
      message_attributes: other.message_attributes_map(),
    }
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
    let endpoint_attributes = DeviceMessageAttributes {
      endpoints: Some(endpoints.to_owned()),
      ..Default::default()
    };
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

#[derive(Debug, Clone, Eq, Hash)]
pub enum ProtocolAttributeIdentifier {
  // The default protocol attribute identifier
  Default,
  Identifier(String),
  Address(String),
}

impl PartialEq for ProtocolAttributeIdentifier {
  fn eq(&self, other: &Self) -> bool {
    use ProtocolAttributeIdentifier::*;
    match (self, other) {
      (Default, Default) => true,
      (Identifier(ident1), Identifier(ident2)) => ident1 == ident2,
      (Address(addr1), Address(addr2)) => addr1 == addr2,
      _ => false,
    }
  }
}

#[derive(Debug, Clone, Getters, MutGetters, Default)]
pub struct ProtocolDeviceConfiguration {
  #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
  specifiers: Vec<ProtocolDeviceSpecifier>,
  #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
  configurations: HashMap<ProtocolAttributeIdentifier, Arc<ProtocolDeviceAttributes>>,
}

impl ProtocolDeviceConfiguration {
  pub fn new(
    specifiers: Vec<ProtocolDeviceSpecifier>,
    configurations: HashMap<ProtocolAttributeIdentifier, Arc<ProtocolDeviceAttributes>>,
  ) -> Self {
    Self {
      specifiers,
      configurations,
    }
  }

  pub fn is_valid(&self) -> Result<(), ButtplugError> {
    for (ident, attrs) in &self.configurations {
      for (message_type, message_attrs) in attrs.message_attributes_map() {
        message_attrs.check(&message_type).map_err(|err| {
          info!("Error in {:?} {}: {:?}", ident, message_type, message_attrs);
          ButtplugError::from(err)
        })?;
      }
    }
    Ok(())
  }

  pub fn device_attributes(
    &self,
    identifier: &ProtocolAttributeIdentifier,
  ) -> Option<&Arc<ProtocolDeviceAttributes>> {
    self.configurations.get(identifier)
  }
}

#[derive(Clone, Debug)]
pub struct DeviceAttributesBuilder {
  allow_raw_messages: bool,
  device_configuration: ProtocolDeviceConfiguration,
}

impl DeviceAttributesBuilder {
  fn new(allow_raw_messages: bool, device_configuration: ProtocolDeviceConfiguration) -> Self {
    Self {
      allow_raw_messages,
      device_configuration,
    }
  }

  pub fn create_from_impl(
    &self,
    device_impl: &Arc<DeviceImpl>,
  ) -> Result<ProtocolDeviceAttributes, ButtplugError> {
    self.create(
      &ProtocolAttributeIdentifier::Address(device_impl.address().to_owned()),
      &ProtocolAttributeIdentifier::Identifier(device_impl.name().to_owned()),
      &device_impl.endpoints(),
    )
  }

  pub fn create(
    &self,
    address: &ProtocolAttributeIdentifier,
    identifier: &ProtocolAttributeIdentifier,
    endpoints: &[Endpoint],
  ) -> Result<ProtocolDeviceAttributes, ButtplugError> {
    let device_attributes = self
      .device_configuration
      .device_attributes(address)
      .or_else(|| self.device_configuration.device_attributes(identifier))
      .or_else(|| {
        self
          .device_configuration
          .device_attributes(&ProtocolAttributeIdentifier::Default)
      })
      .ok_or_else(|| ButtplugError::from(
        ButtplugDeviceError::DeviceConfigurationFileError(format!(
          "Configuration not found for device identifier '{:?}' Address '{:?}'",
          identifier, address
        )),
      ))?;

    let mut attributes = ProtocolDeviceAttributes::new_flattened(device_attributes);

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
pub struct ProtocolBuilder {
  allow_raw_messages: bool,
  creator_func: TryCreateProtocolFunc,
  configuration: ProtocolDeviceConfiguration,
}

impl ProtocolBuilder {
  fn new(
    allow_raw_messages: bool,
    creator_func: TryCreateProtocolFunc,
    configuration: ProtocolDeviceConfiguration,
  ) -> Self {
    Self {
      allow_raw_messages,
      creator_func,
      configuration,
    }
  }

  pub async fn create(
    &self,
    device_impl: Arc<DeviceImpl>,
  ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
    let builder = DeviceAttributesBuilder::new(self.allow_raw_messages, self.configuration.clone());
    (self.creator_func)(device_impl.clone(), builder).await
  }

  pub fn configuration(&self) -> &ProtocolDeviceConfiguration {
    &self.configuration
  }
}

pub struct DeviceConfigurationManager {
  allow_raw_messages: bool,
  protocol_device_configurations: Arc<DashMap<String, ProtocolDeviceConfiguration>>,
  protocol_map: Arc<DashMap<String, TryCreateProtocolFunc>>,
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
    }
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

  pub fn add_protocol<T>(&self, protocol_name: &str) -> Result<(), ButtplugDeviceError>
  where
    T: ButtplugProtocol,
  {
    if !self.protocol_map.contains_key(protocol_name) {
      add_to_protocol_map::<T>(&self.protocol_map, protocol_name);
      Ok(())
    } else {
      Err(ButtplugDeviceError::ProtocolAlreadyAdded(
        protocol_name.to_owned(),
      ))
    }
  }

  pub fn remove_protocol(&self, protocol_name: &str) -> Result<(), ButtplugDeviceError> {
    if self.protocol_map.contains_key(protocol_name) {
      self.protocol_map.remove(protocol_name);
      Ok(())
    } else {
      Err(ButtplugDeviceError::ProtocolNotImplemented(
        protocol_name.to_owned(),
      ))
    }
  }

  pub fn remove_all_protocols(&self) {
    self.protocol_map.clear();
  }

  pub fn protocol_creator(&self, protocol_name: &str) -> Option<TryCreateProtocolFunc> {
    self
      .protocol_map
      .get(protocol_name)
      .map(|pair| *pair.value())
  }

  /// Provides read-only access to the internal protocol/identifier map. Mainly
  /// used for WebBluetooth filter construction, but could also be handy for
  /// listing capabilities in UI, etc.
  pub fn protocol_device_configurations(
    &self,
  ) -> Arc<DashMap<String, ProtocolDeviceConfiguration>> {
    self.protocol_device_configurations.clone()
  }

  pub fn protocol_builder(&self, specifier: &ProtocolDeviceSpecifier) -> Option<ProtocolBuilder> {
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

        let creator_func = self
          .protocol_map
          .get(config.key())
          .map(|pair| *pair.value())?;

        return Some(ProtocolBuilder::new(
          self.allow_raw_messages,
          creator_func,
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
    let specifiers = vec![ProtocolDeviceSpecifier::BluetoothLE(BluetoothLESpecifier {
      names: HashSet::from(["LVS-*".to_owned(), "LovenseDummyTestName".to_owned()]),
      services: HashMap::new(),
      advertised_services: HashSet::new()
    })];
    let mut attributes = HashMap::new();
    attributes.insert(ProtocolAttributeIdentifier::Identifier("P".to_owned()), Arc::new(ProtocolDeviceAttributes::new(Some("Lovense Edge".to_owned()), None, HashMap::new(), None)));
    let pdc = ProtocolDeviceConfiguration::new(specifiers, attributes);
    dcm.add_protocol_device_configuration("lovense", &pdc).unwrap();
    dcm
  }

  #[test]
  fn test_config_equals() {
    let config = create_unit_test_dcm(false);
    let launch =
      ProtocolDeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device("LovenseDummyTestName", &[]));
    assert!(config.protocol_builder(&launch).is_some());
  }

  #[test]
  fn test_config_wildcard_equals() {
    let config = create_unit_test_dcm(false);
    let lovense = ProtocolDeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Whatever",
      &[],
    ));
    assert!(config.protocol_builder(&lovense).is_some());
  }

  #[test]
  #[ignore]
  fn test_specific_device_config_creation() {
    let config = create_unit_test_dcm(false);
    let lovense = ProtocolDeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Whatever",
      &[],
    ));
    let builder = config
      .protocol_builder(&lovense)
      .expect("Test, assuming infallible");
    let config = builder
      .configuration()
      .device_attributes(&ProtocolAttributeIdentifier::Identifier("P".to_owned()))
      .expect("Test, assuming infallible");
    // Make sure we got the right name
    assert_eq!(config.name(), "Lovense Edge");
    // Make sure we overwrote the default of 1
    assert_eq!(
      config
        .message_attributes(&ButtplugDeviceMessageType::VibrateCmd)
        .expect("Test, assuming infallible")
        .feature_count
        .expect("Test, assuming infallible"),
      2
    );
  }

  #[test]
  fn test_raw_device_config_creation() {
    let config = create_unit_test_dcm(true);
    let lovense = ProtocolDeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Whatever",
      &[],
    ));
    let builder = config
      .protocol_builder(&lovense)
      .expect("Test, assuming infallible");
    let device_attr_builder = DeviceAttributesBuilder::new(true, builder.configuration().clone());
    let config = device_attr_builder
      .create(&ProtocolAttributeIdentifier::Address("DoesNotMatter".to_owned()), &ProtocolAttributeIdentifier::Identifier("P".to_owned()), &vec![Endpoint::Tx, Endpoint::Rx])
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
    let lovense = ProtocolDeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Whatever",
      &[],
    ));
    let builder = config
      .protocol_builder(&lovense)
      .expect("Test, assuming infallible");
      let device_attr_builder = DeviceAttributesBuilder::new(false, builder.configuration().clone());
      let config = device_attr_builder
        .create(&ProtocolAttributeIdentifier::Address("DoesNotMatter".to_owned()), &ProtocolAttributeIdentifier::Identifier("P".to_owned()), &vec![Endpoint::Tx, Endpoint::Rx])
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
}
