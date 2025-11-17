use buttplug_server_device_config::UserDeviceIdentifier;
use serde::{Deserialize, Serialize};

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
  },
  DeviceDisconnected {
    index: u32,
  },
  ClientRejected {
    reason: String,
  },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntifaceMessage {
  RequestEngineVersion { expected_version: u32 },
  Stop {},
}
