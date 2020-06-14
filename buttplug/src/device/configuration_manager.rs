// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Device specific identification and protocol implementations.

use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::MessageAttributesMap,
  },
  device::Endpoint,
  util::json::JSONValidator,
};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::mem;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

static DEVICE_CONFIGURATION_JSON: &str =
  include_str!("../../dependencies/buttplug-device-config/buttplug-device-config.json");
static DEVICE_CONFIGURATION_JSON_SCHEMA: &str =
  include_str!("../../dependencies/buttplug-device-config/buttplug-device-config-schema.json");
static USER_DEVICE_CONFIGURATION_JSON_SCHEMA: &str =
  include_str!("../../dependencies/buttplug-device-config/buttplug-user-device-config-schema.json");
static DEVICE_EXTERNAL_CONFIGURATION_JSON: Lazy<Arc<RwLock<Option<String>>>> =
  Lazy::new(|| Arc::new(RwLock::new(None)));
static DEVICE_USER_CONFIGURATION_JSON: Lazy<Arc<RwLock<Option<String>>>> =
  Lazy::new(|| Arc::new(RwLock::new(None)));

pub fn set_external_device_config(config: Option<String>) {
  let mut c = DEVICE_EXTERNAL_CONFIGURATION_JSON.write().unwrap();
  *c = config;
}

pub fn set_user_device_config(config: Option<String>) {
  let mut c = DEVICE_USER_CONFIGURATION_JSON.write().unwrap();
  *c = config;
}

#[allow(dead_code)]
fn clear_user_device_config() {
  let mut c = DEVICE_USER_CONFIGURATION_JSON.write().unwrap();
  *c = None;
}

// Note: There's a ton of extra structs in here just to deserialize the json
// file. Just leave them and build extras (for instance,
// DeviceProtocolConfiguration) if needed elsewhere in the codebase. It's not
// gonna hurt anything and making a ton of serde attributes is just going to get
// confusing (see the messages impl).

#[derive(Deserialize, Debug, Clone)]
pub struct BluetoothLESpecifier {
  pub names: HashSet<String>,
  pub services: HashMap<Uuid, HashMap<Endpoint, Uuid>>,
}

impl PartialEq for BluetoothLESpecifier {
  fn eq(&self, other: &Self) -> bool {
    if self.names.intersection(&other.names).count() > 0 {
      return true;
    }
    for name in &self.names {
      for other_name in &other.names {
        let compare_name: &String;
        let mut wildcard: String;
        if name.ends_with('*') {
          wildcard = name.clone();
          compare_name = &other_name;
        } else if other_name.ends_with('*') {
          wildcard = other_name.clone();
          compare_name = &name;
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
  pub fn new_from_device(name: &str) -> BluetoothLESpecifier {
    let mut set = HashSet::new();
    set.insert(name.to_string());
    BluetoothLESpecifier {
      names: set,
      services: HashMap::new(),
    }
  }
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct XInputSpecifier {
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

#[derive(Deserialize, Debug, PartialEq, Clone, Copy)]
pub struct HIDSpecifier {
  #[serde(rename = "vendor-id")]
  vendor_id: u16,
  #[serde(rename = "product-id")]
  product_id: u16,
}

#[derive(Deserialize, Debug, Clone)]
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

impl PartialEq for SerialSpecifier {
  fn eq(&self, other: &Self) -> bool {
    self.port == other.port
  }
}

#[derive(Deserialize, Debug, PartialEq, Clone, Copy)]
pub struct USBSpecifier {
  #[serde(rename = "vendor-id")]
  vendor_id: u16,
  #[serde(rename = "product-id")]
  product_id: u16,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub enum DeviceSpecifier {
  BluetoothLE(BluetoothLESpecifier),
  HID(HIDSpecifier),
  USB(USBSpecifier),
  Serial(SerialSpecifier),
  XInput(XInputSpecifier),
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProtocolAttributes {
  identifier: Option<Vec<String>>,
  name: Option<HashMap<String, String>>,
  messages: Option<MessageAttributesMap>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProtocolDefinition {
  // Can't get serde flatten specifiers into a String/DeviceSpecifier map, so
  // they're kept separate here, and we return them in get_specifiers(). Feels
  // very clumsy, but we really don't do this a bunch during a session.
  pub usb: Option<Vec<USBSpecifier>>,
  pub btle: Option<BluetoothLESpecifier>,
  pub serial: Option<Vec<SerialSpecifier>>,
  pub hid: Option<Vec<HIDSpecifier>>,
  pub xinput: Option<XInputSpecifier>,
  pub defaults: Option<ProtocolAttributes>,
  pub configurations: Vec<ProtocolAttributes>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct UserProtocolDefinition {
  // Right now, we only allow users to specify serial ports through this
  // interface. It will contain more additions in the future.
  pub serial: Option<Vec<SerialSpecifier>>,
}

fn option_some_eq<T>(a: &Option<T>, b: &T) -> bool
where
  T: PartialEq,
{
  a.as_ref().map_or(false, |x| x == b)
}

fn option_some_eq_vec<T>(a_opt: &Option<Vec<T>>, b: &T) -> bool
where
  T: PartialEq,
{
  a_opt.as_ref().map_or(false, |a_vec| a_vec.contains(b))
}

impl PartialEq<DeviceSpecifier> for ProtocolDefinition {
  fn eq(&self, other: &DeviceSpecifier) -> bool {
    // TODO This seems like a really gross way to do this?
    match other {
      DeviceSpecifier::USB(other_usb) => option_some_eq_vec(&self.usb, other_usb),
      DeviceSpecifier::Serial(other_serial) => option_some_eq_vec(&self.serial, other_serial),
      DeviceSpecifier::BluetoothLE(other_btle) => option_some_eq(&self.btle, other_btle),
      DeviceSpecifier::HID(other_hid) => option_some_eq_vec(&self.hid, other_hid),
      DeviceSpecifier::XInput(other_xinput) => option_some_eq(&self.xinput, other_xinput),
    }
  }
}

#[derive(Deserialize, Debug)]
pub struct ProtocolConfiguration {
  pub(self) protocols: HashMap<String, ProtocolDefinition>,
}

#[derive(Deserialize, Debug)]
pub struct UserProtocolConfiguration {
  pub protocols: HashMap<String, UserProtocolDefinition>,
}

impl ProtocolConfiguration {
  pub fn merge_user_config(&mut self, other: UserProtocolConfiguration) {
    // For now, we're only merging serial info in.
    for (protocol, conf) in other.protocols {
      if self.protocols.contains_key(&protocol) {
        let our_serial_conf_option = &mut self.protocols.get_mut(&protocol).unwrap().serial;
        let mut other_serial_conf = conf.serial;
        if let Some(ref mut our_serial_config) = our_serial_conf_option {
          our_serial_config.extend(other_serial_conf.unwrap());
        } else {
          mem::swap(our_serial_conf_option, &mut other_serial_conf);
        }
      }
    }
  }
}

#[derive(Clone, Debug)]
pub struct DeviceProtocolConfiguration {
  defaults: Option<ProtocolAttributes>,
  configurations: Vec<ProtocolAttributes>,
}

impl DeviceProtocolConfiguration {
  pub fn new(
    defaults: Option<ProtocolAttributes>,
    configurations: Vec<ProtocolAttributes>,
  ) -> Self {
    Self {
      defaults,
      configurations,
    }
  }

  pub fn get_attributes(
    &self,
    identifier: &str,
  ) -> Result<(HashMap<String, String>, MessageAttributesMap), ButtplugError> {
    let mut attributes = MessageAttributesMap::new();
    // If we find defaults, set those up first.
    if let Some(ref attrs) = self.defaults {
      if let Some(ref msg_attrs) = attrs.messages {
        attributes = msg_attrs.clone();
      }
    }
    match self.configurations.iter().find(|attrs| {
      attrs
        .identifier
        .as_ref()
        .unwrap()
        .contains(&identifier.to_owned())
    }) {
      Some(ref attrs) => {
        if let Some(ref msg_attrs) = attrs.messages {
          attributes.extend(msg_attrs.clone());
        }
        Ok((attrs.name.as_ref().unwrap().clone(), attributes))
      }
      None => Err(
        ButtplugDeviceError::new(&format!(
          "Cannot find identifier {} in protocol.",
          identifier
        ))
        .into(),
      ),
    }
  }
}

pub struct DeviceConfigurationManager {
  pub(self) config: ProtocolConfiguration,
}

unsafe impl Send for DeviceConfigurationManager {}
unsafe impl Sync for DeviceConfigurationManager {}

impl Default for DeviceConfigurationManager {
  fn default() -> Self {
    let external_config_guard = DEVICE_EXTERNAL_CONFIGURATION_JSON.clone();
    let external_config = external_config_guard.read().unwrap();
    let mut config: ProtocolConfiguration;
    // TODO We should already load the JSON into the file statics, and just
    // clone it out of our statics as needed.
    let config_validator = JSONValidator::new(DEVICE_CONFIGURATION_JSON_SCHEMA);

    if let Some(ref cfg) = *external_config {
      match config_validator.validate(&cfg) {
        Ok(_) => config = serde_json::from_str(&cfg).unwrap(),
        Err(e) => panic!(
          "Built-in configuration schema is invalid! Aborting! {:?}",
          e
        ),
      }
    } else {
      match config_validator.validate(DEVICE_CONFIGURATION_JSON) {
        Ok(_) => config = serde_json::from_str(DEVICE_CONFIGURATION_JSON).unwrap(),
        Err(e) => panic!(
          "Built-in configuration schema is invalid! Aborting! {:?}",
          e
        ),
      }
    }

    let user_validator = JSONValidator::new(USER_DEVICE_CONFIGURATION_JSON_SCHEMA);
    let user_config_guard = DEVICE_USER_CONFIGURATION_JSON.clone();
    let user_config_str = user_config_guard.read().unwrap();
    if let Some(ref user_cfg) = *user_config_str {
      match user_validator.validate(&user_cfg.to_string()) {
        Ok(_) => config.merge_user_config(serde_json::from_str(&user_cfg.to_string()).unwrap()),
        Err(e) => panic!("User configuration schema is invalid! Aborting! {:?}", e),
      }
    }

    DeviceConfigurationManager { config }
  }
}

impl DeviceConfigurationManager {
  pub fn find_configuration(
    &self,
    specifier: &DeviceSpecifier,
  ) -> Option<(String, ProtocolDefinition)> {
    info!("Looking for protocol that matches spec: {:?}", specifier);
    for (name, def) in self.config.protocols.iter() {
      if def == specifier {
        debug!("Found protocol for spec!");
        return Some((name.clone(), def.clone()));
      }
    }
    info!("No protocol found for spec!");
    None
  }

  pub fn get_protocol_config(&self, name: &str) -> Option<DeviceProtocolConfiguration> {
    info!("Looking for protocol {}", name);
    // TODO It feels like maybe there should be a cleaner way to do this,
    // but I'm not really sure what it is?
    if let Some(proto) = self.config.protocols.get(name) {
      info!("Found a protocol definition for {}", name);
      Some(DeviceProtocolConfiguration::new(
        proto.defaults.clone(),
        proto.configurations.clone(),
      ))
    } else {
      debug!("No matching protocol definition found.");
      None
    }
  }
}

#[cfg(test)]
mod test {
  use super::{
    clear_user_device_config, set_user_device_config, BluetoothLESpecifier,
    DeviceConfigurationManager, DeviceProtocolConfiguration, DeviceSpecifier,
  };
  use crate::core::messages::ButtplugDeviceMessageType;

  #[test]
  fn test_load_config() {
    let config = DeviceConfigurationManager::default();
    debug!("{:?}", config.config);
  }

  #[test]
  fn test_config_equals() {
    let config = DeviceConfigurationManager::default();
    let launch = DeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device("Launch"));
    assert!(config.find_configuration(&launch).is_some());
  }

  #[test]
  fn test_config_wildcard_equals() {
    let config = DeviceConfigurationManager::default();
    let lovense =
      DeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device("LVS-Whatever"));
    assert!(config.find_configuration(&lovense).is_some());
  }

  #[test]
  fn test_specific_device_config_creation() {
    let config = DeviceConfigurationManager::default();
    let lovense =
      DeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device("LVS-Whatever"));
    let proto = config.find_configuration(&lovense).unwrap();
    let proto_config =
      DeviceProtocolConfiguration::new(proto.1.defaults.clone(), proto.1.configurations);
    let (name_map, message_map) = proto_config.get_attributes("P").unwrap();
    // Make sure we got the right name
    assert_eq!(name_map.get("en-us").unwrap(), "Lovense Edge");
    // Make sure we overwrote the default of 1
    assert_eq!(
      message_map
        .get(&ButtplugDeviceMessageType::VibrateCmd)
        .unwrap()
        .feature_count
        .unwrap(),
      2
    );
  }

  #[test]
  fn test_user_config_loading() {
    let mut config = DeviceConfigurationManager::default();
    assert!(config.config.protocols.contains_key("erostek-et312"));
    assert!(config
      .config
      .protocols
      .get("erostek-et312")
      .unwrap()
      .serial
      .as_ref()
      .is_some());
    assert_eq!(
      config
        .config
        .protocols
        .get("erostek-et312")
        .unwrap()
        .serial
        .as_ref()
        .unwrap()
        .len(),
      1
    );
    set_user_device_config(Some(
      r#"
        { 
            "protocols": {
                "erostek-et312": {
                    "serial": [
                        {
                            "port": "COM1",
                            "baud-rate": 19200,
                            "data-bits": 8,
                            "parity": "N",
                            "stop-bits": 1
                        }
                    ]
                }
            }
        }
        "#
      .to_string(),
    ));
    config = DeviceConfigurationManager::default();
    assert!(config.config.protocols.contains_key("erostek-et312"));
    assert!(config
      .config
      .protocols
      .get("erostek-et312")
      .unwrap()
      .serial
      .as_ref()
      .is_some());
    assert_eq!(
      config
        .config
        .protocols
        .get("erostek-et312")
        .unwrap()
        .serial
        .as_ref()
        .unwrap()
        .len(),
      2
    );
    assert!(config
      .config
      .protocols
      .get("erostek-et312")
      .unwrap()
      .serial
      .as_ref()
      .unwrap()
      .iter()
      .any(|x| x.port == "COM1"));
    clear_user_device_config();
  }
}
