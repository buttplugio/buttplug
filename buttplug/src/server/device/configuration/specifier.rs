// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::Endpoint;
use getset::{Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

// Note: There's a ton of extra structs in here just to deserialize the json
// file. Just leave them and build extras (for instance,
// DeviceProtocolConfiguration) if needed elsewhere in the codebase. It's not
// gonna hurt anything and making a ton of serde attributes is just going to get
// confusing (see the messages impl).

#[derive(Serialize, Deserialize, Debug, Clone, Getters, MutGetters, Setters, Eq)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
pub struct BluetoothLEManufacturerData {
  company: u16,
  data: Option<Vec<u8>>,
}

impl BluetoothLEManufacturerData {
  pub fn new(company: u16, data: &Option<Vec<u8>>) -> Self {
    Self {
      company,
      data: data.clone(),
    }
  }
}

impl PartialEq for BluetoothLEManufacturerData {
  fn eq(&self, other: &Self) -> bool {
    if self.company != *other.company() {
      return false;
    }

    // Only the deserialized device config can have none has a data value. If it does, that means
    // all we care about is the company value, at which point we can return true here.
    if self.data.is_none() || other.data.is_none() {
      return true;
    }

    let data = self.data().as_ref().expect("Already checked existence");
    let other_data = other.data().as_ref().expect("Already checked existence");

    if data.len() == other_data.len() {
      if *data == *other_data {
        return true;
      }
      return false;
    }

    // If our data lengths are different, see if one is a subsequence of the other.
    //
    // Since we don't have the information to know if either self or other is what was loaded from
    // the JSON file, we have to blindly guess for needle and haystack here based on length. This
    // means if manufacturer data comes back from a device shorter than the data the device config
    // expects, we could have a false match. This needs to be contexualized but that would require a
    // large device config system rewrite at the moment, so we'll just live with the bug for now.
    let needle = if data.len() < other_data.len() {
      data
    } else {
      other_data
    };
    let mut haystack = if data.len() > other_data.len() {
      data.as_slice()
    } else {
      other_data.as_slice()
    };
    while !haystack.is_empty() {
      if haystack.starts_with(needle) {
        return true;
      }
      haystack = &haystack[1..];
    }

    false
  }
}

/// Specifier for Bluetooth LE Devices
///
/// Used by protocols for identifying bluetooth devices via their advertisements, as well as
/// defining the services and characteristics they are expected to have.
#[derive(Serialize, Deserialize, Debug, Clone, Getters, MutGetters, Setters)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
pub struct BluetoothLESpecifier {
  /// Set of expected advertised names for this device.
  names: HashSet<String>,
  /// Array of possible manufacturer data values.
  #[serde(default, rename = "manufacturer-data")]
  manufacturer_data: Vec<BluetoothLEManufacturerData>,
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
    // If names or manufacturer data are found, use those automatically.
    if self.names.intersection(&other.names).count() > 0 {
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

    if !self.manufacturer_data.is_empty() && !other.manufacturer_data.is_empty() {
      for data in &self.manufacturer_data {
        if other.manufacturer_data.contains(data) {
          return true;
        }
      }
    }

    if self
      .advertised_services
      .intersection(&other.advertised_services)
      .count()
      > 0
    {
      return true;
    }

    false
  }
}

impl BluetoothLESpecifier {
  pub fn new(
    names: HashSet<String>,
    manufacturer_data: Vec<BluetoothLEManufacturerData>,
    advertised_services: HashSet<Uuid>,
    services: HashMap<Uuid, HashMap<Endpoint, Uuid>>,
  ) -> Self {
    Self {
      names,
      manufacturer_data,
      advertised_services,
      services,
    }
  }

  /// Creates a specifier from a BLE device advertisement.
  pub fn new_from_device(
    name: &str,
    manufacturer_data: &HashMap<u16, Vec<u8>>,
    advertised_services: &[Uuid],
  ) -> BluetoothLESpecifier {
    let mut name_set = HashSet::new();
    name_set.insert(name.to_string());
    let mut data_vec = vec![];
    for (company, data) in manufacturer_data.iter() {
      data_vec.push(BluetoothLEManufacturerData::new(
        *company,
        &Some(data.clone()),
      ));
    }
    let service_set = HashSet::from_iter(advertised_services.iter().copied());
    BluetoothLESpecifier {
      names: name_set,
      manufacturer_data: data_vec,
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
    self.advertised_services = self
      .advertised_services
      .union(&other.advertised_services)
      .cloned()
      .collect();
    self.services.extend(other.services);
  }
}

/// Specifier for [Lovense Connect
/// Service](crate::server::device::communication_manager::lovense_connect_service) devices
///
/// Network based services, has no attributes because the [Lovense Connect
/// Service](crate::server::device::communication_manager::lovense_connect_service) device communication manager
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

/// Specifier for [XInput](crate::server::device::communication_manager::xinput) devices
///
/// Network based services, has no attributes because the
/// [XInput](crate::server::device::communication_manager::xinput) device communication manager handles all device
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
#[derive(
  Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, Getters, Setters, MutGetters,
)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
pub struct HIDSpecifier {
  #[serde(rename = "vendor-id")]
  vendor_id: u16,
  #[serde(rename = "product-id")]
  product_id: u16,
}

impl HIDSpecifier {
  pub fn new(vendor_id: u16, product_id: u16) -> Self {
    Self {
      vendor_id,
      product_id,
    }
  }
}

/// Specifier for Serial devices
///
/// Handles serial port device identification (via port names) and configuration.
#[derive(Serialize, Deserialize, Debug, Clone, Default, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
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
#[derive(
  Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, Getters, Setters, MutGetters,
)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
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
  names: HashSet<String>,
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
  pub fn new(names: &Vec<String>) -> WebsocketSpecifier {
    let mut set = HashSet::new();
    for name in names {
      set.insert(name.clone());
    }
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

impl Eq for ProtocolCommunicationSpecifier {
}
