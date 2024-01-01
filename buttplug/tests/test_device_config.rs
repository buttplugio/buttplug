// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;
extern crate buttplug;

use buttplug::server::ButtplugServerBuilder;

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
    "major": 2,
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
    "major": 2,
    "minor": 999
  },
  "user-configs": {}
}
"#;

#[cfg(feature = "server")]
#[tokio::test]
async fn test_valid_null_version_config() {
  ButtplugServerBuilder::default()
    .user_device_configuration_json(Some(BASE_VALID_VERSION_CONFIG_JSON.to_owned()))
    .finish()
    .unwrap();
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_valid_null_user_config() {
  ButtplugServerBuilder::default()
    .user_device_configuration_json(Some(BASE_VALID_NULL_USER_CONFIG_JSON.to_owned()))
    .finish()
    .unwrap();
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_invalid_null_version_config() {
  assert!(ButtplugServerBuilder::default()
    .user_device_configuration_json(Some(BASE_INVALID_VERSION_CONFIG_JSON.to_owned()))
    .finish()
    .is_err());
}

#[cfg(feature = "server")]
#[tokio::test]
#[ignore = "Still need to update for new message format"]
async fn test_basic_device_config() {
  ButtplugServerBuilder::default()
    .device_configuration_json(Some(BASE_CONFIG_JSON.to_owned()))
    .finish()
    .unwrap();
}

#[cfg(feature = "server")]
#[tokio::test]
#[ignore = "Still need to update for new message format"]
async fn test_valid_step_range() {
  let user_config_json = r#"
  {
    "version": 63,
    "user-configs": {
      "devices": {
            "test-addr": {
              "messages": {
                "VibrateCmd": {
                  "StepRange": [
                    [50, 60]
                  ]
                }
              }
            }
          }
    }
  }
  "#;
  assert!(ButtplugServerBuilder::default()
    .device_configuration_json(Some(BASE_CONFIG_JSON.to_owned()))
    .user_device_configuration_json(Some(user_config_json.to_owned()))
    .finish()
    .is_ok());
}

#[cfg(feature = "server")]
#[tokio::test]
#[ignore = "Still need to update for new message format"]
async fn test_invalid_step_range_device_config_wrong_range_length() {
  let user_config_json = r#"
  {
    "version": 63,
    "user-configs": {
      "devices": {
            "test-addr": {
              "messages": {
                "VibrateCmd": {
                  "StepRange": [
                    [50]
                  ]
                }
              }
            }
      }
    }
  }
  "#;
  assert!(ButtplugServerBuilder::default()
    .device_configuration_json(Some(BASE_CONFIG_JSON.to_owned()))
    .user_device_configuration_json(Some(user_config_json.to_owned()))
    .finish()
    .is_err());
}

#[cfg(feature = "server")]
#[tokio::test]
#[ignore = "Still need to update for new message format"]
async fn test_invalid_step_range_device_config_wrong_order() {
  let user_config_json = r#"
  {
    "version": 63,
    "user-configs": {
      "devices": {
            "test-addr": {
              "messages": {
                "VibrateCmd": {
                  "StepRange": [
                    [60, 50]
                  ]
                }
              }
            }
          }

    }
  }
  "#;
  assert!(ButtplugServerBuilder::default()
    .device_configuration_json(Some(BASE_CONFIG_JSON.to_owned()))
    .user_device_configuration_json(Some(user_config_json.to_owned()))
    .finish()
    .is_ok());
  assert!(ButtplugServerBuilder::default()
    .device_configuration_json(Some(BASE_CONFIG_JSON.to_owned()))
    .user_device_configuration_json(Some(user_config_json.to_owned()))
    .finish()
    .is_ok());
}
