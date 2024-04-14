// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;
extern crate buttplug;

use buttplug::server::ButtplugServerBuilder;
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
  assert!(load_protocol_configs(None, Some(BASE_VALID_VERSION_CONFIG_JSON.to_owned()), false).is_ok());
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_valid_null_user_config() {
  assert!(load_protocol_configs(None, Some(BASE_VALID_NULL_USER_CONFIG_JSON.to_owned()), false).is_ok());
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_invalid_null_version_config() {
  assert!(load_protocol_configs(None, Some(BASE_INVALID_VERSION_CONFIG_JSON.to_owned()), false).is_err());
}

#[cfg(feature = "server")]
#[tokio::test]
#[ignore = "Still need to update for new message format"]
async fn test_basic_device_config() {
  assert!(load_protocol_configs(Some(BASE_CONFIG_JSON.to_owned()), None, false).is_ok());
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
  assert!(load_protocol_configs(Some(BASE_CONFIG_JSON.to_owned()), Some(user_config_json.to_owned()), false).is_ok());
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
  assert!(load_protocol_configs(Some(BASE_CONFIG_JSON.to_owned()), Some(user_config_json.to_owned()), false).is_err());
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
  assert!(load_protocol_configs(Some(BASE_CONFIG_JSON.to_owned()), Some(user_config_json.to_owned()), false).is_err());
}

/*
#[tokio::test]
async fn test_server_builder_null_device_config() {
  let mut builder = ButtplugServerBuilder::default();
  let _ = builder
    .device_configuration_json(None)
    .finish()
    .expect("Test, assuming infallible.");
}

#[tokio::test]
async fn test_server_builder_device_config_invalid_json() {
  let mut builder = ButtplugServerBuilder::default();
  assert!(builder
    .device_configuration_json(Some("{\"Not Valid JSON\"}".to_owned()))
    .finish()
    .is_err());
}

#[tokio::test]
async fn test_server_builder_device_config_schema_break() {
  let mut builder = ButtplugServerBuilder::default();
  // missing version block.
  let device_json = r#"{
      "protocols": {
        "jejoue": {
          "btle": {
            "names": [
              "Je Joue"
            ],
            "services": {
              "0000fff0-0000-1000-8000-00805f9b34fb": {
                "tx": "0000fff1-0000-1000-8000-00805f9b34fb"
              }
            }
          },
          "defaults": {
            "name": {
              "en-us": "Je Joue Device"
            },
            "messages": {
              "VibrateCmd": {
                "FeatureCount": 2,
                "StepCount": [
                  5,
                  5
                ]
              }
            }
          }
        },
      }
    }"#;
  assert!(builder
    .device_configuration_json(Some(device_json.to_owned()))
    .finish()
    .is_err());
}

#[tokio::test]
async fn test_server_builder_device_config_old_config_version() {
  let mut builder = ButtplugServerBuilder::default();
  // missing version block.
  let device_json = r#"{
      "version": 0,
      "protocols": {}
    }
    "#;
  assert!(builder
    .device_configuration_json(Some(device_json.to_owned()))
    .finish()
    .is_err());
}

#[tokio::test]
async fn test_server_builder_null_user_device_config() {
  let mut builder = ButtplugServerBuilder::default();
  let _ = builder
    .user_device_configuration_json(None)
    .finish()
    .expect("Test, assuming infallible.");
}

#[tokio::test]
async fn test_server_builder_user_device_config_invalid_json() {
  let mut builder = ButtplugServerBuilder::default();
  assert!(builder
    .user_device_configuration_json(Some("{\"Not Valid JSON\"}".to_owned()))
    .finish()
    .is_err());
}

#[tokio::test]
async fn test_server_builder_user_device_config_schema_break() {
  let mut builder = ButtplugServerBuilder::default();
  // missing version block.
  let device_json = r#"{
      "protocols": {
        "jejoue": {
          "btle": {
            "names": [
              "Je Joue"
            ],
            "services": {
              "0000fff0-0000-1000-8000-00805f9b34fb": {
                "tx": "0000fff1-0000-1000-8000-00805f9b34fb"
              }
            }
          },
          "defaults": {
            "name": {
              "en-us": "Je Joue Device"
            },
            "messages": {
              "VibrateCmd": {
                "FeatureCount": 2,
                "StepCount": [
                  5,
                  5
                ]
              }
            }
          }
        },
      }
    }"#;
  assert!(builder
    .user_device_configuration_json(Some(device_json.to_owned()))
    .finish()
    .is_err());
}

#[tokio::test]
#[ignore = "Skip until we've figured out whether we actually want version differences to fail."]
async fn test_server_builder_user_device_config_old_config_version() {
  let mut builder = ButtplugServerBuilder::default();
  // missing version block.
  let device_json = r#"{
      "version": 0,
      "protocols": {}
    }
    "#;
  assert!(builder
    .user_device_configuration_json(Some(device_json.to_owned()))
    .finish()
    .is_err());
}
*/