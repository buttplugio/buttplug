// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Device specific identification and protocol implementations.

use super::protocol::ButtplugProtocol;
use super::protocols::lovense::LovenseProtocol;
use crate::{
    core::{errors::ButtplugError, messages::MessageAttributes, errors::ButtplugDeviceError},
    device::Endpoint,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

// Note: There's a ton of extra structs in here just to deserialize the json
// file. Just leave them and build extras (for instance,
// DeviceProtocolConfiguraation) if needed elsewhere in the codebase. It's not
// gonna hurt anything and making a ton of serde attributes is just going to get
// confusing (see the messages impl).

const DEVICE_CONFIGURATION_FILE: &str =
    include_str!("../../dependencies/buttplug-device-config/buttplug-device-config.json");

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
                if name.ends_with("*") {
                    wildcard = name.clone();
                    compare_name = &other_name;
                } else if other_name.ends_with("*") {
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
    ports: HashSet<String>,
}

impl PartialEq for SerialSpecifier {
    fn eq(&self, other: &Self) -> bool {
        self.ports.intersection(&other.ports).count() > 0
    }
}

#[derive(Deserialize, Debug, PartialEq, Clone, Copy)]
pub struct USBSpecifier {
    #[serde(rename = "vendor-id")]
    vendor_id: u16,
    #[serde(rename = "product-id")]
    product_id: u16,
}

#[derive(Deserialize, Debug, PartialEq)]
pub enum DeviceSpecifier {
    BluetoothLE(BluetoothLESpecifier),
    HID(HIDSpecifier),
    USB(USBSpecifier),
    Serial(SerialSpecifier),
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProtocolAttributes {
    identifier: Option<Vec<String>>,
    name: Option<HashMap<String, String>>,
    messages: Option<HashMap<String, MessageAttributes>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ProtocolDefinition {
    // Can't get serde flatten specifiers into a String/DeviceSpecifier map, so
    // they're kept separate here, and we return them in get_specifiers(). Feels
    // very clumsy, but we really don't do this a bunch during a session.
    pub usb: Option<USBSpecifier>,
    pub btle: Option<BluetoothLESpecifier>,
    pub serial: Option<SerialSpecifier>,
    pub hid: Option<HIDSpecifier>,
    pub defaults: Option<ProtocolAttributes>,
    pub configurations: Vec<ProtocolAttributes>,
}

fn option_some_eq<T>(a: &Option<T>, b: &T) -> bool
where
    T: PartialEq,
{
    match &a {
        Some(a) => a == b,
        _ => false,
    }
}

impl PartialEq<DeviceSpecifier> for ProtocolDefinition {
    fn eq(&self, other: &DeviceSpecifier) -> bool {
        // TODO This seems like a really gross way to do this?
        match other {
            DeviceSpecifier::USB(other_usb) => option_some_eq(&self.usb, other_usb),
            DeviceSpecifier::Serial(other_serial) => option_some_eq(&self.serial, other_serial),
            DeviceSpecifier::BluetoothLE(other_btle) => option_some_eq(&self.btle, other_btle),
            DeviceSpecifier::HID(other_hid) => option_some_eq(&self.hid, other_hid),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ProtocolConfiguration {
    protocols: HashMap<String, ProtocolDefinition>,
}

#[derive(Clone, Debug)]
pub struct DeviceProtocolConfiguration {
    defaults: Option<ProtocolAttributes>,
    configurations: Vec<ProtocolAttributes>,
}

impl DeviceProtocolConfiguration {
    pub fn new(defaults: Option<ProtocolAttributes>, configurations: Vec<ProtocolAttributes>) -> Self {
        Self {
            defaults,
            configurations
        }
    }

    pub fn get_attributes(&self, identifier: &str) -> Result<(HashMap<String, String>, HashMap<String, MessageAttributes>), ButtplugError> {
        let mut attributes = HashMap::<String, MessageAttributes>::new();
        // If we find defaults, set those up first.
        if let Some(ref attrs) = self.defaults {
            if let Some(ref msg_attrs) = attrs.messages {
                attributes = msg_attrs.clone();
            }
        }
        match self.configurations.iter().find(|attrs| attrs.identifier.as_ref().unwrap().contains(&identifier.to_owned())) {
            Some(ref attrs) => {
                if let Some(ref msg_attrs) = attrs.messages {
                    attributes.extend(msg_attrs.clone());
                }
                Ok((attrs.name.as_ref().unwrap().clone(), attributes))
            },
            None => Err(ButtplugDeviceError::new(&format!("Cannot find identifier {} in protocol.", identifier)).into())
        }
    }
}

type ProtocolConstructor = Box<dyn Fn(DeviceProtocolConfiguration) -> Box<dyn ButtplugProtocol>>;

pub struct DeviceConfigurationManager {
    config: ProtocolConfiguration,
    protocols: HashMap<String, ProtocolConstructor>,
}

unsafe impl Send for DeviceConfigurationManager {}
unsafe impl Sync for DeviceConfigurationManager {}

impl DeviceConfigurationManager {
    pub fn load_from_internal() -> DeviceConfigurationManager {
        let config = serde_json::from_str(DEVICE_CONFIGURATION_FILE).unwrap();
        // Do not try to use HashMap::new() here. We need the explicit typing,
        // otherwise we'll just get an anonymous closure type during insert that
        // won't match.
        let mut protocols = HashMap::<String, ProtocolConstructor>::new();
        protocols.insert(
            "lovense".to_owned(),
            Box::new(|config: DeviceProtocolConfiguration| Box::new(LovenseProtocol::new(config))),
        );
        DeviceConfigurationManager { config, protocols }
    }

    pub fn find_protocol(
        &self,
        specifier: &DeviceSpecifier,
    ) -> Option<(String, ProtocolDefinition)> {
        for (name, def) in self.config.protocols.iter() {
            if def == specifier {
                return Some((name.clone(), def.clone()));
            }
        }
        None
    }

    pub fn create_protocol_impl(
        &self,
        name: &String,
    ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
        match self.config.protocols.get(name) {
            Some(proto) => {
                Ok(self.protocols.get(name).unwrap()(DeviceProtocolConfiguration::new(proto.defaults.clone(), proto.configurations.clone())))
            },
            None => {
                Err(ButtplugDeviceError::new(&format!("No protocol named {} available", name)).into())
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::{BluetoothLESpecifier, DeviceConfigurationManager, DeviceSpecifier, DeviceProtocolConfiguration};

    #[test]
    fn test_load_config() {
        let _ = env_logger::builder().is_test(true).try_init();
        let config = DeviceConfigurationManager::load_from_internal();
        debug!("{:?}", config.config);
    }

    #[test]
    fn test_config_equals() {
        let _ = env_logger::builder().is_test(true).try_init();
        let config = DeviceConfigurationManager::load_from_internal();
        let launch = DeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device("Launch"));
        assert!(config.find_protocol(&launch).is_some());
    }

    #[test]
    fn test_config_wildcard_equals() {
        let _ = env_logger::builder().is_test(true).try_init();
        let config = DeviceConfigurationManager::load_from_internal();
        let lovense =
            DeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device("LVS-Whatever"));
        assert!(config.find_protocol(&lovense).is_some());
    }

    #[test]
    fn test_specific_device_config_creation() {
        let _ = env_logger::builder().is_test(true).try_init();
        let config = DeviceConfigurationManager::load_from_internal();
        let lovense =
            DeviceSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device("LVS-Whatever"));
        let proto = config.find_protocol(&lovense).unwrap();
        let proto_config = DeviceProtocolConfiguration::new(proto.1.defaults.clone(), proto.1.configurations.clone());
        let (name_map, message_map) = proto_config.get_attributes("P").unwrap();
        // Make sure we got the right name
        assert_eq!(name_map.get("en-us").unwrap(), "Lovense Edge");
        // Make sure we overwrote the default of 1
        assert_eq!(message_map.get("VibrateCmd").unwrap().feature_count.unwrap(), 2);
    }
}
