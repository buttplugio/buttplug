// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, ButtplugMessage, Clone, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct RequestServerInfo {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub(super) id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "ClientName"))]
    pub client_name: String,
    // Default for this message is set to 0, as this field didn't exist in the
    // first version of the protocol.
    #[cfg_attr(feature = "serialize_json", serde(rename = "MessageVersion"), serde(default))]
    pub message_version: u32,
}

impl RequestServerInfo {
    pub fn new(client_name: &str, message_version: u32) -> Self {
        Self {
            id: 1,
            client_name: client_name.to_string(),
            message_version,
        }
    }
}

#[cfg(test)]
mod test {
    use super::RequestServerInfo;

    #[cfg(feature = "serialize_json")]
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
            message_version: 2
        };
        assert_eq!(serde_json::from_str::<RequestServerInfo>(new_json).unwrap(), new_msg);
    }

    #[cfg(feature = "serialize_json")]
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
            message_version: 0
        };
        assert_eq!(serde_json::from_str::<RequestServerInfo>(old_json).unwrap(), old_msg);
    }
}