use getset::{Getters, MutGetters, Setters};
use serde::{Serialize, Deserialize};


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
#[derive(
  Debug, Eq, PartialEq, Hash, Clone, Getters, MutGetters, Serialize, Deserialize,
)]
#[getset(get = "pub(crate)", get_mut = "pub(crate)")]
pub struct UserDeviceIdentifier {
  /// Name of the protocol used
  protocol: String,
  /// Internal identifier for the protocol used
  attributes_identifier: Option<String>,
  /// Address, as possibly serialized by whatever the managing library for the Device Communication Manager is.
  address: String,
}

impl UserDeviceIdentifier {
  /// Creates a new instance
  pub fn new(address: &str, protocol: &str, identifier: &Option<String>) -> Self {
    Self {
      address: address.to_owned(),
      protocol: protocol.to_owned(),
      attributes_identifier: identifier.clone(),
    }
  }
}

/// Set of information used for matching devices to their features and related communication protocol.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Getters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub(crate)", get_mut = "pub(crate)")]
pub struct BaseDeviceIdentifier {
  /// Name of the protocol this device uses to communicate
  protocol: String,
  /// Some([identifier]) if there's an identifier, otherwise None if default
  attributes_identifier: Option<String>,
}

impl BaseDeviceIdentifier {
  pub fn new(
    protocol: &str,
    attributes_identifier: &Option<String>,
  ) -> Self {
    Self {
      protocol: protocol.to_owned(),
      attributes_identifier: attributes_identifier.clone(),
    }
  }
}

impl From<&UserDeviceIdentifier> for BaseDeviceIdentifier {
  fn from(other: &UserDeviceIdentifier) -> Self {
    Self {
      protocol: other.protocol().clone(),
      attributes_identifier: other.attributes_identifier().clone(),
    }
  }
}

impl PartialEq<UserDeviceIdentifier> for BaseDeviceIdentifier {
  fn eq(&self, other: &UserDeviceIdentifier) -> bool {
    self.protocol == *other.protocol()
      && self.attributes_identifier == *other.attributes_identifier()
  }
}
