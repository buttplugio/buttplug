// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::{
  errors::ButtplugMessageError,
  message::{
    ButtplugMessage,
    ButtplugMessageFinalizer,
    ButtplugMessageSpecVersion,
    ButtplugMessageValidator,
  },
};
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

fn return_version0() -> ButtplugMessageSpecVersion {
  ButtplugMessageSpecVersion::Version0
}

// For RequestServerInfo, serde will take care of invalid message versions from json, and internal
// representations of versions require using the version enum as a type bound. Therefore we do not
// need explicit content checking for the message.
#[derive(
  Debug,
  ButtplugMessage,
  ButtplugMessageFinalizer,
  Clone,
  PartialEq,
  Eq,
  Getters,
  CopyGetters,
  Serialize,
  Deserialize,
)]
pub struct RequestServerInfoV1 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "ClientName")]
  #[getset(get = "pub")]
  client_name: String,
  // Default for this message is set to 0, as this field didn't exist in the
  // first version of the protocol.
  #[serde(rename = "MessageVersion", default = "return_version0")]
  #[getset(get_copy = "pub")]
  message_version: ButtplugMessageSpecVersion,
}

impl RequestServerInfoV1 {
  pub fn new(client_name: &str, message_version: ButtplugMessageSpecVersion) -> Self {
    Self {
      id: 1,
      client_name: client_name.to_string(),
      message_version,
    }
  }
}

impl ButtplugMessageValidator for RequestServerInfoV1 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}

#[cfg(test)]
mod test {
  use super::{ButtplugMessageSpecVersion, RequestServerInfoV1};

  #[test]
  fn test_request_server_info_version1_json_conversion() {
    let new_json = r#"
{
        "Id": 1,
        "ClientName": "Test Client",
        "MessageVersion": 2
}
        "#;
    let new_msg = RequestServerInfoV1 {
      id: 1,
      client_name: "Test Client".to_owned(),
      message_version: ButtplugMessageSpecVersion::Version2,
    };
    assert_eq!(
      serde_json::from_str::<RequestServerInfoV1>(new_json).expect("Test unwrap"),
      new_msg
    );
  }

  #[test]
  fn test_request_server_info_version0_json_conversion() {
    let old_json = r#"
{
        "Id": 1,
        "ClientName": "Test Client"
}
        "#;
    let old_msg = RequestServerInfoV1 {
      id: 1,
      client_name: "Test Client".to_owned(),
      message_version: ButtplugMessageSpecVersion::Version0,
    };
    assert_eq!(
      serde_json::from_str::<RequestServerInfoV1>(old_json).expect("Test unwrap"),
      old_msg
    );
  }
}
