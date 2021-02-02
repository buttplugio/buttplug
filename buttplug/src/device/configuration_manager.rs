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
    messages::{ButtplugDeviceMessageType, DeviceMessageAttributes, DeviceMessageAttributesMap},
  },
  device::Endpoint,
  util::json::JSONValidator,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::mem;
use uuid::Uuid;

static DEVICE_CONFIGURATION_JSON: &str =
  include_str!("../../buttplug-device-config/buttplug-device-config.json");
static DEVICE_CONFIGURATION_JSON_SCHEMA: &str =
  include_str!("../../buttplug-device-config/buttplug-device-config-schema.json");
static USER_DEVICE_CONFIGURATION_JSON_SCHEMA: &str =
  include_str!("../../buttplug-device-config/buttplug-user-device-config-schema.json");

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

#[derive(Deserialize, Debug, Clone, Default)]
pub struct SerialSpecifier {
  #[serde(rename = "baud-rate")]
  pub baud_rate: u32,
  #[serde(rename = "data-bits")]
  pub data_bits: u8,
  #[serde(rename = "stop-bits")]
  pub stop_bits: u8,
  pub parity: char,
  pub port: String,
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
  messages: Option<DeviceMessageAttributesMap>,
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
  #[serde(default)]
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
  pub version: u32,
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
  allow_raw_messages: bool,
  defaults: Option<ProtocolAttributes>,
  configurations: Vec<ProtocolAttributes>,
}

impl DeviceProtocolConfiguration {
  pub fn new(
    allow_raw_messages: bool,
    defaults: Option<ProtocolAttributes>,
    configurations: Vec<ProtocolAttributes>,
  ) -> Self {
    Self {
      allow_raw_messages,
      defaults,
      configurations,
    }
  }

  pub fn get_attributes(
    &self,
    identifier: &str,
    endpoints: &[Endpoint],
  ) -> Result<(HashMap<String, String>, DeviceMessageAttributesMap), ButtplugError> {
    let mut attributes = DeviceMessageAttributesMap::new();

    // If we find defaults, set those up first.
    if let Some(ref attrs) = self.defaults {
      if let Some(ref msg_attrs) = attrs.messages {
        attributes = msg_attrs.clone();
      }
    }

    // If we're allowing raw messages, tack those on beforehand also.
    if self.allow_raw_messages {
      let endpoint_attributes = DeviceMessageAttributes {
        endpoints: Some(endpoints.to_owned()),
        ..Default::default()
      };
      attributes.insert(
        ButtplugDeviceMessageType::RawReadCmd,
        endpoint_attributes.clone(),
      );
      attributes.insert(
        ButtplugDeviceMessageType::RawWriteCmd,
        endpoint_attributes.clone(),
      );
      attributes.insert(
        ButtplugDeviceMessageType::RawSubscribeCmd,
        endpoint_attributes.clone(),
      );
      attributes.insert(
        ButtplugDeviceMessageType::RawUnsubscribeCmd,
        endpoint_attributes,
      );
    }

    let device_attrs = if let Some(attrs) = self.configurations.iter().find(|attrs| {
      attrs
        .identifier
        .as_ref()
        .unwrap()
        .contains(&identifier.to_owned())
    }) {
      attrs
    } else if let Some(attrs) = &self.defaults {
      // If we can't find an identifier but we have a default block, return that.
      attrs
    } else {
      // If we can't find anything, give up.
      return Err(
        ButtplugDeviceError::ProtocolAttributesNotFound(format!(
          "Cannot find identifier {} in protocol.",
          identifier
        ))
      .into());
    };

    if let Some(ref msg_attrs) = device_attrs.messages {
      attributes.extend(msg_attrs.clone());
    }
    
    // Everything needs to be able to stop.
    attributes
      .entry(ButtplugDeviceMessageType::StopDeviceCmd)
      .or_insert_with(DeviceMessageAttributes::default);

    // The device config JSON schema requires us to have a name map, so we can unwrap this.
    Ok((device_attrs.name.as_ref().unwrap().clone(), attributes))
  }
}

pub struct DeviceConfigurationManager {
  allow_raw_messages: bool,
  pub(self) config: ProtocolConfiguration,
}

impl Default for DeviceConfigurationManager {
  fn default() -> Self {
    // Unwrap allowed here because we assume our built in device config will
    // always work. System won't pass tests or possibly even build otherwise.
    Self::new_with_options(false, &None, &None).unwrap()
  }
}

impl DeviceConfigurationManager {
  pub fn new_with_options(
    allow_raw_messages: bool,
    external_config: &Option<String>,
    user_config: &Option<String>,
  ) -> Result<Self, ButtplugDeviceError> {
    // TODO Handling references incorrectly here.
    let config_str = if let Some(cfg) = external_config {
      cfg
    } else {
      DEVICE_CONFIGURATION_JSON
    };

    let config_validator = JSONValidator::new(DEVICE_CONFIGURATION_JSON_SCHEMA);
    let mut config: ProtocolConfiguration = match config_validator.validate(&config_str) {
      Ok(_) => match serde_json::from_str(&config_str) {
        Ok(protocol_config) => protocol_config,
        Err(err) => {
          return Err(ButtplugDeviceError::DeviceConfigurationFileError(format!(
            "{}",
            err
          )))
        }
      },
      Err(err) => {
        return Err(ButtplugDeviceError::DeviceConfigurationFileError(format!(
          "{}",
          err
        )))
      }
    };
    info!(
      "Successfully loaded Device Configuration File Version {}",
      config.version
    );

    if let Some(user_config_str) = user_config {
      let user_validator = JSONValidator::new(USER_DEVICE_CONFIGURATION_JSON_SCHEMA);
      match user_validator.validate(&user_config_str) {
        Ok(_) => match serde_json::from_str(&user_config_str) {
          Ok(user_cfg) => config.merge_user_config(user_cfg),
          Err(err) => {
            return Err(ButtplugDeviceError::DeviceConfigurationFileError(format!(
              "{}",
              err
            )))
          }
        },
        Err(err) => {
          return Err(ButtplugDeviceError::DeviceConfigurationFileError(format!(
            "{}",
            err
          )))
        }
      }
    }

    Ok(DeviceConfigurationManager {
      allow_raw_messages,
      config,
    })
  }

  /// Provides read-only access to the internal protocol/identifier map. Mainly
  /// used for WebBluetooth filter construction, but could also be handy for
  /// listing capabilities in UI, etc.
  pub fn protocol_configurations(&self) -> &HashMap<String, ProtocolDefinition> {
    &self.config.protocols
  }

  pub fn find_configuration(
    &self,
    specifier: &DeviceSpecifier,
  ) -> Option<(bool, String, ProtocolDefinition)> {
    info!("Looking for protocol that matches specifier: {:?}", specifier);
    for (name, def) in self.config.protocols.iter() {
      if def == specifier {
        debug!("Found protocol {:?} for specifier {:?}.", name, specifier);
        return Some((self.allow_raw_messages, name.clone(), def.clone()));
      }
    }
    info!("No protocol found for specifier {:?}.", specifier);
    None
  }

  pub fn get_protocol_config(&self, name: &str) -> Option<DeviceProtocolConfiguration> {
    info!("Looking for protocol {}", name);
    // TODO It feels like maybe there should be a cleaner way to do this,
    // but I'm not really sure what it is?
    if let Some(proto) = self.config.protocols.get(name) {
      info!("Found a protocol definition for {}", name);
      Some(DeviceProtocolConfiguration::new(
        self.allow_raw_messages,
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
    BluetoothLESpecifier,
    DeviceConfigurationManager,
    DeviceProtocolConfiguration,
    DeviceSpecifier,
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
      DeviceProtocolConfiguration::new(false, proto.2.defaults.clone(), proto.2.configurations);
    let (name_map, message_map) = proto_config.get_attributes("P", &vec![]).unwrap();
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
  fn test_raw_device_config_creation() {
    let config = DeviceConfigurationManager::new_with_options(true, &None, &None).unwrap();
    let lovense =
      DeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device("LVS-Whatever"));
    let proto = config.find_configuration(&lovense).unwrap();
    let proto_config =
      DeviceProtocolConfiguration::new(true, proto.2.defaults.clone(), proto.2.configurations);
    let (name_map, message_map) = proto_config.get_attributes("P", &vec![]).unwrap();
    // Make sure we got the right name
    assert_eq!(name_map.get("en-us").unwrap(), "Lovense Edge");
    // Make sure we overwrote the default of 1
    assert!(message_map.contains_key(&ButtplugDeviceMessageType::RawWriteCmd));
    assert!(message_map.contains_key(&ButtplugDeviceMessageType::RawReadCmd));
    assert!(message_map.contains_key(&ButtplugDeviceMessageType::RawSubscribeCmd));
    assert!(message_map.contains_key(&ButtplugDeviceMessageType::RawUnsubscribeCmd));
  }

  #[test]
  fn test_non_raw_device_config_creation() {
    let config = DeviceConfigurationManager::default();
    let lovense =
      DeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device("LVS-Whatever"));
    let proto = config.find_configuration(&lovense).unwrap();
    let proto_config =
      DeviceProtocolConfiguration::new(false, proto.2.defaults.clone(), proto.2.configurations);
    let (name_map, message_map) = proto_config.get_attributes("P", &vec![]).unwrap();
    // Make sure we got the right name
    assert_eq!(name_map.get("en-us").unwrap(), "Lovense Edge");
    // Make sure we overwrote the default of 1
    assert!(!message_map.contains_key(&ButtplugDeviceMessageType::RawWriteCmd));
    assert!(!message_map.contains_key(&ButtplugDeviceMessageType::RawReadCmd));
    assert!(!message_map.contains_key(&ButtplugDeviceMessageType::RawSubscribeCmd));
    assert!(!message_map.contains_key(&ButtplugDeviceMessageType::RawUnsubscribeCmd));
  }

  #[test]
  fn test_user_config_loading() {
    let mut config = DeviceConfigurationManager::default();
    assert!(config.config.protocols.contains_key("nobra"));
    assert!(config
      .config
      .protocols
      .get("nobra")
      .unwrap()
      .serial
      .as_ref()
      .is_some());
    assert_eq!(
      config
        .config
        .protocols
        .get("nobra")
        .unwrap()
        .serial
        .as_ref()
        .unwrap()
        .len(),
      1
    );
    config = DeviceConfigurationManager::new_with_options(
      false,
      &None,
      &Some(
        r#"
        { 
            "protocols": {
                "nobra": {
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
      ),
    )
    .unwrap();
    assert!(config.config.protocols.contains_key("nobra"));
    assert!(config
      .config
      .protocols
      .get("nobra")
      .unwrap()
      .serial
      .as_ref()
      .is_some());
    assert_eq!(
      config
        .config
        .protocols
        .get("nobra")
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
      .get("nobra")
      .unwrap()
      .serial
      .as_ref()
      .unwrap()
      .iter()
      .any(|x| x.port == "COM1"));
  }

  // TODO Test invalid config load (not json)
  // TODO Test invalid user config load (not json)
  // TODO Test device config with repeated ble service
  // TODO Test device config with repeated ble characteristic
  // TODO Test user config with invalid protocol
  // TODO Test user config with invalid bus type
  // TODO Test user config with conflicting BLE name
}
