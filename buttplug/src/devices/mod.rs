// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Device specific identification and protocol implementations.

use crate::core::messages::MessageAttributes;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const DeviceConfigurationFile: &str = include_str!("../../dependencies/buttplug-device-config/buttplug-device-config.json");

#[derive(Deserialize, Debug)]
pub struct BluetoothLESpecifier {
    names: Vec<String>,
    services: HashMap<Uuid, HashMap<String, Uuid>>
}

#[derive(Deserialize, Debug)]
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
    ports: Vec<String>
}

#[derive(Deserialize, Debug)]
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
