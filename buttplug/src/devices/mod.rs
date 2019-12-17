// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Device specific identification and protocol implementations.

use crate::core::messages::MessageAttributes;
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const DeviceConfigurationFile: &str = include_str!("../../dependencies/buttplug-device-config/buttplug-device-config.json");

#[derive(Deserialize, Debug)]
pub struct BluetoothLESpecifier {
    pub names: HashSet<String>,
    pub services: HashMap<Uuid, HashMap<String, Uuid>>
}

impl PartialEq for BluetoothLESpecifier {
    fn eq(&self, other: &Self) -> bool {
        self.names.intersection(&other.names).count() > 0
    }
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct HIDSpecifier {
    #[serde(rename = "vendor-id")]
    vendor_id: u16,
    #[serde(rename = "product-id")]
    product_id: u16
}

#[derive(Deserialize, Debug)]
pub struct SerialSpecifier {
    #[serde(rename = "baud-rate")]
    baud_rate: u32,
    #[serde(rename = "data-bits")]
    data_bits: u8,
    #[serde(rename = "stop-bits")]
    stop_bits: u8,
    parity: char,
    ports: HashSet<String>
}

impl PartialEq for SerialSpecifier {
    fn eq(&self, other: &Self) -> bool {
        self.ports.intersection(&other.ports).count() > 0
    }
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct USBSpecifier {
    #[serde(rename = "vendor-id")]
    vendor_id: u16,
    #[serde(rename = "product-id")]
    product_id: u16
}

#[derive(Deserialize, Debug)]
struct ProtocolAttributes {
    identifier: Option<Vec<String>>,
    name: Option<HashMap<String, String>>,
    messages: Option<HashMap<String, MessageAttributes>>
}

#[derive(Deserialize, Debug)]
struct ProtocolDefinition {
    #[serde(rename = "btle")]
    bluetooth_le: Option<BluetoothLESpecifier>,
    #[serde(rename = "hid")]
    hid: Option<HIDSpecifier>,
    #[serde(rename = "usb")]
    usb: Option<USBSpecifier>,
    #[serde(rename = "serial")]
    serial: Option<SerialSpecifier>,
    defaults: Option<ProtocolAttributes>,
    configurations: Vec<ProtocolAttributes>
}

fn option_some_eq<T>(a: &Option<T>, b: &Option<T>) -> bool
where T: PartialEq {
    match (&a, &b) {
        (Some(a), Some(b)) => a == b,
        _ => false
    }
}

impl PartialEq for ProtocolDefinition {
    fn eq(&self, other: &Self) -> bool {
        option_some_eq(&self.bluetooth_le, &other.bluetooth_le) ||
        option_some_eq(&self.hid, &other.hid) ||
        option_some_eq(&self.serial, &other.serial) ||
        option_some_eq(&self.usb, &other.usb)
    }
}

#[derive(Deserialize, Debug)]
struct ProtocolConfiguration {
    protocols: HashMap<String, ProtocolDefinition>
}

struct DeviceConfigurationManager {
}

impl DeviceConfigurationManager {
    pub fn new() -> DeviceConfigurationManager {
        DeviceConfigurationManager {
        }
    }

    pub fn load() {
        let config: ProtocolConfiguration = serde_json::from_str(DeviceConfigurationFile).unwrap();
        println!("{:?}", config);
    }
}


#[cfg(test)]
mod test {
    use super::DeviceConfigurationManager;

    #[test]
    fn test_load_config() {
        DeviceConfigurationManager::load();
    }
}
