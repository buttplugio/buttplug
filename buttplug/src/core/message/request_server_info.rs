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

fn return_version0() -> ButtplugMessageSpecVersion {
  ButtplugMessageSpecVersion::Version0
}
#[derive(
  Debug, ButtplugMessage, ButtplugMessageFinalizer, Clone, PartialEq, Eq, Getters, CopyGetters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct RequestServerInfo {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ClientName"))]
  #[getset(get = "pub")]
  client_name: String,
  // Default for this message is set to 0, as this field didn't exist in the
  // first version of the protocol.
  #[cfg_attr(
    feature = "serialize-json",
    serde(rename = "MessageVersion"),
    serde(default = "return_version0")
  )]
  #[getset(get_copy = "pub")]
  message_version: ButtplugMessageSpecVersion,
}

impl RequestServerInfo {
  pub fn new(client_name: &str, message_version: ButtplugMessageSpecVersion) -> Self {
    Self {
      id: 1,
      client_name: client_name.to_string(),
      message_version,
    }
  }
}

impl ButtplugMessageValidator for RequestServerInfo {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}

#[cfg(test)]
mod test {
  use super::{ButtplugMessageSpecVersion, RequestServerInfo};

  #[cfg(feature = "serialize-json")]
  #[test]
  fn test_request_server_info_version1_json_conversion() {
    let new_json = r#"
{
        "Id": 1,
        "ClientName": "Test Client",
        "MessageVersion": 2
}
        "#;
    let new_msg = RequestServerInfo {
      id: 1,
      client_name: "Test Client".to_owned(),
      message_version: ButtplugMessageSpecVersion::Version2,
    };
    assert_eq!(
      serde_json::from_str::<RequestServerInfo>(new_json).expect("Test unwrap"),
      new_msg
    );
  }

  #[cfg(feature = "serialize-json")]
  #[test]
  fn test_request_server_info_version0_json_conversion() {
    let old_json = r#"
{
        "Id": 1,
        "ClientName": "Test Client"
}
        "#;
    let old_msg = RequestServerInfo {
      id: 1,
      client_name: "Test Client".to_owned(),
      message_version: ButtplugMessageSpecVersion::Version0,
    };
    assert_eq!(
      serde_json::from_str::<RequestServerInfo>(old_json).expect("Test unwrap"),
      old_msg
    );
  }
}
