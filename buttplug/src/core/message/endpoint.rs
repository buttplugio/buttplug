use serde::{
  de::{self, Visitor},
  Deserialize,
  Deserializer,
  Serialize,
  Serializer,
};
use std::{
  fmt::{self, Debug},
  str::FromStr,
  string::ToString,
};

use core::hash::Hash;

// We need this array to be exposed in our WASM FFI, but the only way to do that
// is to expose it at the declaration level. Therefore, we use the WASM feature
// to assume we're building for WASM and attach our bindgen. The serde
// de/serialization is taken care of at the FFI level.

/// Endpoint names for device communication.
///
/// Endpoints denote different contextual communication targets on a device. For instance, for a
/// device that uses UART style communication (serial, a lot of Bluetooth LE devices, etc...) most
/// devices will just have a Tx and Rx endpoint. However, on other devices that can have varying
/// numbers of endpoints and configurations (USB, Bluetooth LE, etc...) we add some names with more
/// context. These names are used in [Device Configuration](crate::server::device::configuration)
/// and the [Device Configuration File](crate::util::device_configuration), and are expected to
/// de/serialize to lowercase versions of their names.
#[derive(EnumString, Clone, Debug, PartialEq, Eq, Hash, Display, Copy)]
#[strum(serialize_all = "lowercase")]
pub enum Endpoint {
  /// Expect to take commands, when multiple receive endpoints may be available
  Command,
  /// Firmware updates (Buttplug does not update firmware, but some firmware endpoints are used for
  /// mode setting)
  Firmware,
  /// Common receive endpoint name
  Rx,
  /// Receive endpoint for accelerometer data
  RxAccel,
  /// Receive endpoint for battery levels (usually expected to be BLE standard profile)
  RxBLEBattery,
  /// Receive endpoint for BLE model (usually expected to be BLE standard profile)
  RxBLEModel,
  /// Receive endpoint for pressure sensors
  RxPressure,
  /// Receive endpoint for touch sensors
  RxTouch,
  /// Common transmit endpoint name
  Tx,
  /// Transmit endpoint for hardware mode setting.
  TxMode,
  /// Transmit endpoint for shock setting (unused)
  TxShock,
  /// Transmit endpoint for vibration setting
  TxVibrate,
  /// Transmit endpoint for vendor (proprietary) control
  TxVendorControl,
  /// Transmit endpoint for whitelist updating
  Whitelist,
  /// Generic endpoint (available for user configurations)
  Generic0,
  /// Generic endpoint (available for user configurations)
  Generic1,
  /// Generic endpoint (available for user configurations)
  Generic2,
  /// Generic endpoint (available for user configurations)
  Generic3,
  /// Generic endpoint (available for user configurations)
  Generic4,
  /// Generic endpoint (available for user configurations)
  Generic5,
  /// Generic endpoint (available for user configurations)
  Generic6,
  /// Generic endpoint (available for user configurations)
  Generic7,
  /// Generic endpoint (available for user configurations)
  Generic8,
  /// Generic endpoint (available for user configurations)
  Generic9,
  /// Generic endpoint (available for user configurations)
  Generic10,
  /// Generic endpoint (available for user configurations)
  Generic11,
  /// Generic endpoint (available for user configurations)
  Generic12,
  /// Generic endpoint (available for user configurations)
  Generic13,
  /// Generic endpoint (available for user configurations)
  Generic14,
  /// Generic endpoint (available for user configurations)
  Generic15,
  /// Generic endpoint (available for user configurations)
  Generic16,
  /// Generic endpoint (available for user configurations)
  Generic17,
  /// Generic endpoint (available for user configurations)
  Generic18,
  /// Generic endpoint (available for user configurations)
  Generic19,
  /// Generic endpoint (available for user configurations)
  Generic20,
  /// Generic endpoint (available for user configurations)
  Generic21,
  /// Generic endpoint (available for user configurations)
  Generic22,
  /// Generic endpoint (available for user configurations)
  Generic23,
  /// Generic endpoint (available for user configurations)
  Generic24,
  /// Generic endpoint (available for user configurations)
  Generic25,
  /// Generic endpoint (available for user configurations)
  Generic26,
  /// Generic endpoint (available for user configurations)
  Generic27,
  /// Generic endpoint (available for user configurations)
  Generic28,
  /// Generic endpoint (available for user configurations)
  Generic29,
  /// Generic endpoint (available for user configurations)
  Generic30,
  /// Generic endpoint (available for user configurations)
  Generic31,
}

// Implement to/from string serialization for Endpoint struct
impl Serialize for Endpoint {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(&self.to_string())
  }
}

struct EndpointVisitor;

impl<'de> Visitor<'de> for EndpointVisitor {
  type Value = Endpoint;

  fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    formatter.write_str("a string representing an endpoint")
  }

  fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
  where
    E: de::Error,
  {
    Endpoint::from_str(value).map_err(|e| E::custom(format!("{}", e)))
  }
}

impl<'de> Deserialize<'de> for Endpoint {
  fn deserialize<D>(deserializer: D) -> Result<Endpoint, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer.deserialize_str(EndpointVisitor)
  }
}
