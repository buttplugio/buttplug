// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use compact_str::CompactString;
use getset::{Getters, MutGetters};
use serde::{Deserialize, Serialize};

/// Identifying information for devices that are currently connected or have connected in the past.
///
/// Contains the 3 fields needed to uniquely identify a device in the system. Unlike
/// [ConfigurationDeviceIdentifier]s, [UserDeviceIdentifier] will always have a device address
/// available.
///
/// NOTE: UserDeviceIdentifiers are NOT portable across platforms. For instance, bluetooth addresses
/// are used for the address field on bluetooth devices. These will differ between all platforms due
/// to address formatting as well as available information (macOS/iOS and WebBluetooth obfuscate
/// bluetooth addresses)
#[derive(Debug, Eq, PartialEq, Hash, Clone, Getters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub", get_mut = "pub(crate)")]
pub struct UserDeviceIdentifier {
  /// Name of the protocol used
  protocol: CompactString,
  /// Internal identifier for the protocol used
  identifier: Option<CompactString>,
  /// Address, as possibly serialized by whatever the managing library for the Device Communication Manager is.
  address: CompactString,
}

impl UserDeviceIdentifier {
  /// Creates a new instance
  pub fn new(address: &str, protocol: &str, identifier: Option<&str>) -> Self {
    Self {
      address: address.into(),
      protocol: protocol.into(),
      identifier: identifier.map(|s| s.into())
    }
  }
}

/// Set of information used for matching devices to their features and related communication protocol.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Getters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub(crate)", get_mut = "pub(crate)")]
pub struct BaseDeviceIdentifier {
  /// Name of the protocol this device uses to communicate
  protocol: CompactString,
  /// Some([identifier]) if there's an identifier, otherwise None if default
  identifier: Option<CompactString>,
}

impl BaseDeviceIdentifier {
  pub fn new_default(protocol: &str) -> Self {
    Self::new(protocol, &None)
  }

  pub fn new_with_identifier(protocol: &str, attributes_identifier: CompactString) -> Self {
    Self {
      protocol: protocol.into(),
      identifier: Some(attributes_identifier),
    }
  }

  pub fn new(protocol: &str, attributes_identifier: &Option<CompactString>) -> Self {
    Self {
      protocol: protocol.into(),
      identifier: attributes_identifier.clone(),
    }
  }
}

impl From<&UserDeviceIdentifier> for BaseDeviceIdentifier {
  fn from(other: &UserDeviceIdentifier) -> Self {
    Self {
      protocol: other.protocol().clone(),
      identifier: other.identifier().clone(),
    }
  }
}

impl PartialEq<UserDeviceIdentifier> for BaseDeviceIdentifier {
  fn eq(&self, other: &UserDeviceIdentifier) -> bool {
    self.protocol == *other.protocol() && self.identifier == *other.identifier()
  }
}
