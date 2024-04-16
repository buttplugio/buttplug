// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;
extern crate buttplug;

use buttplug::util::device_configuration::load_protocol_configs;

const BASE_CONFIG_JSON: &str = r#"
{
  "version": 999,
  "protocols": {
    "kiiroo-v21": {
      "btle": {
        "names": [
          "OhMiBod LUMEN"
        ],
        "services": {
          "00001900-0000-1000-8000-00805f9b34fb": {
            "whitelist": "00001901-0000-1000-8000-00805f9b34fb",
            "tx": "00001902-0000-1000-8000-00805f9b34fb",
            "rx": "00001903-0000-1000-8000-00805f9b34fb"
          },
          "a0d70001-4c16-4ba7-977a-d394920e13a3": {
            "tx": "a0d70002-4c16-4ba7-977a-d394920e13a3",
            "rx": "a0d70003-4c16-4ba7-977a-d394920e13a3"
          }
        }
      },
      "defaults": {
        "name": "Kiiroo V2.1 Device",
        "messages": {}
      },
      "configurations": [
        {
          "identifier": [
            "OhMiBod LUMEN"
          ],
          "name": "OhMiBod Lumen",
          "messages": {
            "ScalarCmd": [
              {
                "ActuatorType": "Vibrate",
                "StepRange": [0, 100]
              }
            ]
          }
        }
      ]
    }
  }
}
"#;

const BASE_VALID_VERSION_CONFIG_JSON: &str = r#"
{
  "version": {
    "major": 3,
    "minor": 999
  }
}
"#;

const BASE_INVALID_VERSION_CONFIG_JSON: &str = r#"
{
  "version": {
    "major": 999,
    "minor": 999
  }
}
"#;

const BASE_VALID_NULL_USER_CONFIG_JSON: &str = r#"
{
  "version": {
    "major": 3,
    "minor": 999
  },
  "user-configs": {}
}
"#;

#[cfg(feature = "server")]
#[tokio::test]
async fn test_valid_null_version_config() {
  assert!(
    load_protocol_configs(&None, &Some(BASE_VALID_VERSION_CONFIG_JSON.to_owned()), false).is_ok()
  );
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_valid_null_user_config() {
  assert!(load_protocol_configs(
    &None,
    &Some(BASE_VALID_NULL_USER_CONFIG_JSON.to_owned()),
    false
  )
  .is_ok());
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_invalid_null_version_config() {
  assert!(load_protocol_configs(
    &None,
    &Some(BASE_INVALID_VERSION_CONFIG_JSON.to_owned()),
    false
  )
  .is_err());
}

#[cfg(feature = "server")]
#[tokio::test]
#[ignore = "Still need to update for new message format"]
async fn test_basic_device_config() {
  assert!(load_protocol_configs(&Some(BASE_CONFIG_JSON.to_owned()), &None, false).is_ok());
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_valid_user_config() {
  let user_config_json = r#"
  {
    "version": {
      major: 3,
      minor: 0
    },
    "user-configs": {
      "devices": [
        {
          "identifier": {
            "address": "UserConfigTest",
            "protocol": "lovense",
            "identifier": "B"
          },
          "config": {
            "name": "Lovense Test Device",
            "features": [
              {
                "feature-type": "Vibrate",
                "description": "Test Speed",
                "actuator": {
                  "step-range": [
                    0,
                    20
                  ],
                  "step-limit: [
                    10,
                    15
                  ],
                  "messages": [
                    "ScalarCmd"
                  ]
                }
              },
              {
                "feature-type": "Battery",
                "description": "Battery Level",
                "sensor": {
                  "value-range": [
                    [
                      0,
                      100
                    ]
                  ],
                  "messages": [
                    "SensorReadCmd"
                  ]
                }
              }
            ],
            "user-config": {
              "allow": false,
              "deny": false,
              "index": 0,
              "display-name": "Lovense Name Test"
            }
          }
        }
      ]
    }
  }"#;
  assert!(load_protocol_configs(
    &Some(BASE_CONFIG_JSON.to_owned()),
    &Some(user_config_json.to_owned()),
    false
  )
  .is_err());
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_invalid_step_range_device_config_wrong_range_length() {
  let user_config_json = r#"
  {
    "version": {
      major: 3,
      minor: 0
    },
    "user-configs": {
      "devices": [
        {
          "identifier": {
            "address": "UserConfigTest",
            "protocol": "lovense",
            "identifier": "B"
          },
          "config": {
            "name": "Lovense Test Device",
            "features": [
              {
                "feature-type": "Vibrate",
                "description": "Test Speed",
                "actuator": {
                  "step-range": [
                    10
                  ],
                  "messages": [
                    "ScalarCmd"
                  ]
                }
              },
              {
                "feature-type": "Battery",
                "description": "Battery Level",
                "sensor": {
                  "value-range": [
                    [
                      0,
                      100
                    ]
                  ],
                  "messages": [
                    "SensorReadCmd"
                  ]
                }
              }
            ],
            "user-config": {
              "allow": false,
              "deny": false,
              "index": 0,
              "display-name": "Lovense Name Test"
            }
          }
        }
      ]
    }
  }
  "#;
  assert!(load_protocol_configs(
    &Some(BASE_CONFIG_JSON.to_owned()),
    &Some(user_config_json.to_owned()),
    false
  )
  .is_err());
}

#[tokio::test]
async fn test_server_builder_null_device_config() {
  assert!(load_protocol_configs(&None, &None, false).is_ok())
}

#[tokio::test]
async fn test_server_builder_device_config_invalid_json() {
  assert!(load_protocol_configs(&Some("{\"Not Valid JSON\"}".to_owned()), &None, false).is_err())
}

#[tokio::test]
#[ignore = "Not testing the right thing"]
async fn test_server_builder_device_config_old_config_version() {
  // missing version block.
  let device_json = r#"{
      "version": {
        "major": 1,
        "minor": 0
      },
      "protocols": {}
    }
    "#;
  assert!(load_protocol_configs(&Some(device_json.to_owned()), &None, false).is_err());
}

#[tokio::test]
async fn test_server_builder_user_device_config_old_config_version() {
  // missing version block.
  let device_json = r#"{
      "version": {
        "major": 1,
        "minor": 0
      },
      "protocols": {}
    }
    "#;
  assert!(load_protocol_configs(&None, &Some(device_json.to_owned()), false).is_err());
}

#[tokio::test]
async fn test_server_builder_user_device_config_invalid_json() {
  assert!(load_protocol_configs(&None, &Some("{\"Not Valid JSON\"}".to_owned()), false).is_err())
}
