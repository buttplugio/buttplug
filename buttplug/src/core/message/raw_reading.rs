// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

// This message can have an Id of 0, as it can be emitted as part of a
// subscription and won't have a matching task Id in that case.
#[derive(
  Debug,
  ButtplugDeviceMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  PartialEq,
  Eq,
  Clone,
  Getters,
  CopyGetters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct RawReading {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Endpoint"))]
  #[getset(get_copy = "pub")]
  endpoint: Endpoint,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Data"))]
  #[getset(get = "pub")]
  data: Vec<u8>,
}

impl RawReading {
  pub fn new(device_index: u32, endpoint: Endpoint, data: Vec<u8>) -> Self {
    Self {
      id: 0,
      device_index,
      endpoint,
      data,
    }
  }
}

#[cfg(feature = "serialize-json")]
#[cfg(test)]
mod test {
  use crate::core::message::{ButtplugCurrentSpecServerMessage, Endpoint, RawReading};

  #[test]
  fn test_endpoint_deserialize() {
    let endpoint_str =
      "{\"RawReading\":{\"Id\":0,\"DeviceIndex\":0,\"Endpoint\":\"tx\",\"Data\":[0]}}";
    let union: ButtplugCurrentSpecServerMessage =
      serde_json::from_str(endpoint_str).expect("Infallible deserialization.");
    assert_eq!(
      ButtplugCurrentSpecServerMessage::RawReading(RawReading::new(0, Endpoint::Tx, vec!(0))),
      union
    );
  }

  #[test]
  fn test_endpoint_serialize() {
    let union =
      ButtplugCurrentSpecServerMessage::RawReading(RawReading::new(0, Endpoint::Tx, vec![0]));
    let js = serde_json::to_string(&union).expect("Infallible serialization.");
    let endpoint_str =
      "{\"RawReading\":{\"Id\":0,\"DeviceIndex\":0,\"Endpoint\":\"tx\",\"Data\":[0]}}";
    assert_eq!(js, endpoint_str);
  }
}
