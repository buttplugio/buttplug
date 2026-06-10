// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_server_device_config::UserDeviceIdentifier;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskListEntry {
  pub id: u64,
  pub path: String,
  pub detached: bool,
}

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
  EngineServerCreated {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    service_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    instance_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    txt_records: Option<Vec<String>>,
  },
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
  TaskStarted {
    id: u64,
    path: String,
  },
  TaskEnded {
    id: u64,
    path: String,
    /// How the task ended: "Completed" | "Cancelled" | "Panicked"
    /// (the Debug rendering of `buttplug_core`'s `TaskOutcome`).
    outcome: String,
  },
  TaskList {
    tasks: Vec<TaskListEntry>,
  },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntifaceMessage {
  RequestEngineVersion { expected_version: u32 },
  Stop {},
  RequestTaskList {},
}

#[cfg(test)]
mod test {
  use super::{EngineErrorDetail, EngineMessage, IntifaceMessage};
  use serde_json::json;

  #[test]
  fn test_task_message_serialization() {
    let msg = EngineMessage::TaskEnded {
      id: 42,
      path: "server-1/ping-timer/timer".to_owned(),
      outcome: "Cancelled".to_owned(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let back: EngineMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, EngineMessage::TaskEnded { id: 42, .. }));

    let req = r#"{"RequestTaskList":{}}"#;
    let parsed: IntifaceMessage = serde_json::from_str(req).unwrap();
    assert!(matches!(parsed, IntifaceMessage::RequestTaskList {}));
  }

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

  #[test]
  fn engine_server_created_serializes_optional_service_metadata() {
    let message = EngineMessage::EngineServerCreated {
      service_type: Some("_intiface_engine._tcp".to_owned()),
      instance_name: Some("Intiface ABC123".to_owned()),
      port: Some(12345),
      txt_records: Some(vec!["path=/".to_owned()]),
    };

    assert_eq!(
      serde_json::to_value(message).unwrap(),
      json!({
        "EngineServerCreated": {
          "service_type": "_intiface_engine._tcp",
          "instance_name": "Intiface ABC123",
          "port": 12345,
          "txt_records": ["path=/"]
        }
      }),
    );
  }

  #[test]
  fn engine_server_created_deserializes_legacy_empty_payload() {
    let message: EngineMessage =
      serde_json::from_value(json!({"EngineServerCreated": {}})).unwrap();

    assert!(matches!(
      message,
      EngineMessage::EngineServerCreated {
        service_type: None,
        instance_name: None,
        port: None,
        txt_records: None,
      }
    ));
  }
}
