// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_server_device_config::UserDeviceIdentifier;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum EngineErrorDetail {
  PortInUse { address: String, port: u16 },
}

// Everything in this struct is an object, even if it has null contents. This is to make other
// languages happy when trying to recompose JSON into objects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineMessage {
  EngineVersion {
    version: String,
  },
  EngineStarted {},
  EngineError {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<EngineErrorDetail>,
  },
  EngineServerCreated {},
  EngineStopped {},
  ClientConnected {
    client_name: String,
  },
  ClientDisconnected {},
  DeviceConnected {
    name: String,
    index: u32,
    identifier: UserDeviceIdentifier,
    #[serde(skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    needs_keepalive: bool,
  },
  DeviceDisconnected {
    index: u32,
  },
  ClientRejected {
    reason: String,
  },
  DeviceOutputObservation {
    device_index: u32,
    feature_index: u32,
    output_type: String,
    value: f64,
  },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntifaceMessage {
  RequestEngineVersion { expected_version: u32 },
  Stop {},
}

#[cfg(test)]
mod test {
  use super::{EngineErrorDetail, EngineMessage};
  use serde_json::json;

  #[test]
  fn generic_engine_error_serializes_without_structured_fields() {
    let message = EngineMessage::EngineError {
      error: "startup failed".to_owned(),
      detail: None,
    };

    assert_eq!(
      serde_json::to_value(message).unwrap(),
      json!({"EngineError": {"error": "startup failed"}}),
    );
  }

  #[test]
  fn port_in_use_engine_error_serializes_structured_fields() {
    let message = EngineMessage::EngineError {
      error: "address already in use".to_owned(),
      detail: Some(EngineErrorDetail::PortInUse {
        address: "127.0.0.1".to_owned(),
        port: 12345,
      }),
    };

    assert_eq!(
      serde_json::to_value(message).unwrap(),
      json!({
        "EngineError": {
          "error": "address already in use",
          "detail": {
            "code": "port_in_use",
            "port": 12345,
            "address": "127.0.0.1"
          }
        }
      }),
    );
  }
}
