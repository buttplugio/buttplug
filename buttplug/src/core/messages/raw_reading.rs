// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
use crate::device::Endpoint;
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct RawReading {
  #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
  pub(super) id: u32,
  #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
  pub device_index: u32,
  #[cfg_attr(feature = "serialize_json", serde(rename = "Endpoint"))]
  pub endpoint: Endpoint,
  #[cfg_attr(feature = "serialize_json", serde(rename = "Data"))]
  pub data: Vec<u8>,
}

impl RawReading {
  pub fn new(device_index: u32, endpoint: Endpoint, data: Vec<u8>) -> Self {
    Self {
      id: 1,
      device_index,
      endpoint,
      data,
    }
  }
}

#[cfg(feature = "serialize_json")]
#[cfg(test)]
mod test {
  use crate::core::messages::{ButtplugCurrentSpecServerMessage, RawReading};
  use crate::device::Endpoint;

  #[test]
  fn test_endpoint_deserialize() {
    let endpoint_str =
      "{\"RawReading\":{\"Id\":1,\"DeviceIndex\":0,\"Endpoint\":\"tx\",\"Data\":[0]}}";
    let union: ButtplugCurrentSpecServerMessage = serde_json::from_str(&endpoint_str).unwrap();
    assert_eq!(
      ButtplugCurrentSpecServerMessage::RawReading(RawReading::new(0, Endpoint::Tx, vec!(0))),
      union
    );
  }

  #[test]
  fn test_endpoint_serialize() {
    let union = ButtplugCurrentSpecServerMessage::RawReading(RawReading::new(0, Endpoint::Tx, vec![0]));
    let js = serde_json::to_string(&union).unwrap();
    let endpoint_str =
      "{\"RawReading\":{\"Id\":1,\"DeviceIndex\":0,\"Endpoint\":\"tx\",\"Data\":[0]}}";
    assert_eq!(js, endpoint_str);
  }
}
