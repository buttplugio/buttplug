// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::{
  ButtplugDeviceMessage,
  ButtplugMessage,
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
  Endpoint,
};
use getset::{CopyGetters, Getters};
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
  Serialize,
  Deserialize,
)]
pub struct RawReadingV2 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[serde(rename = "Endpoint")]
  #[getset(get_copy = "pub")]
  endpoint: Endpoint,
  #[serde(rename = "Data")]
  #[getset(get = "pub")]
  data: Vec<u8>,
}

impl RawReadingV2 {
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
  use crate::core::message::{ButtplugServerMessageCurrent, Endpoint, RawReadingV2};

  #[test]
  fn test_endpoint_deserialize() {
    let endpoint_str =
      "{\"RawReading\":{\"Id\":0,\"DeviceIndex\":0,\"Endpoint\":\"tx\",\"Data\":[0]}}";
    let union: ButtplugServerMessageCurrent =
      serde_json::from_str(endpoint_str).expect("Infallible deserialization.");
    assert_eq!(
      ButtplugServerMessageCurrent::RawReading(RawReadingV2::new(0, Endpoint::Tx, vec!(0))),
      union
    );
  }

  #[test]
  fn test_endpoint_serialize() {
    let union =
      ButtplugServerMessageCurrent::RawReading(RawReadingV2::new(0, Endpoint::Tx, vec![0]));
    let js = serde_json::to_string(&union).expect("Infallible serialization.");
    let endpoint_str =
      "{\"RawReading\":{\"Id\":0,\"DeviceIndex\":0,\"Endpoint\":\"tx\",\"Data\":[0]}}";
    assert_eq!(js, endpoint_str);
  }
}
