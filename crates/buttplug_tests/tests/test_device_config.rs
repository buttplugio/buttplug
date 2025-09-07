// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;

use buttplug_server_device_config::load_protocol_configs;
use tokio_test::assert_ok;

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
    "major": 4,
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
    "major": 4,
    "minor": 999
  },
  "user_configs": {}
}
"#;


#[tokio::test]
async fn test_valid_null_version_config() {
  assert_ok!(load_protocol_configs(
    &Some(BASE_VALID_VERSION_CONFIG_JSON.to_owned()),
    &None,
    false
  ));
}


#[tokio::test]
async fn test_valid_null_user_config() {
  assert_ok!(load_protocol_configs(
    &None,
    &Some(BASE_VALID_NULL_USER_CONFIG_JSON.to_owned()),
    false
  ));
}


#[tokio::test]
async fn test_invalid_null_version_config() {
  assert!(load_protocol_configs(
    &None,
    &Some(BASE_INVALID_VERSION_CONFIG_JSON.to_owned()),
    false
  )
  .is_err());
}


#[tokio::test]
#[ignore = "Still need to update for new message format"]
async fn test_basic_device_config() {
  assert!(load_protocol_configs(&Some(BASE_CONFIG_JSON.to_owned()), &None, false).is_ok());
}


#[tokio::test]
async fn test_valid_user_config() {
  let user_config_json = r#"
  {
    "version": {
      major: 3,
      minor: 0
    },
    "user_configs": {
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
                "description": "Test Speed",
                "actuator": {
                  "step_range": [
                    0,
                    20
                  ],
                  "step_limit: [
                    10,
                    15
                  ],
                  "messages": [
                    "ScalarCmd"
                  ]
                }
              },
              {
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
            "user_config": {
              "allow": false,
              "deny": false,
              "index": 0,
              "display_name": "Lovense Name Test"
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


#[tokio::test]
async fn test_invalid_step_range_device_config_wrong_range_length() {
  let user_config_json = r#"
  {
    "version": {
      major: 3,
      minor: 0
    },
    "user_configs": {
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
            "user_config": {
              "allow": false,
              "deny": false,
              "index": 0,
              "display_name": "Lovense Name Test"
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

/*
    #[tokio::test]
    fn test_user_config_loading() {
      // Assume we have a nobra's entry in the device config.
      let mut config = create_test_dcm(false);
      assert!(config.protocol_definitions().contains_key("nobra"));
      assert!(config
        .protocol_definitions()
        .get("nobra")
        .expect("Test, assuming infallible")
        .serial
        .as_ref()
        .is_some());
      assert_eq!(
        config
          .protocol_definitions()
          .get("nobra")
          .expect("Test, assuming infallible")
          .serial
          .as_ref()
          .expect("Test, assuming infallible")
          .len(),
        1
      );

      // Now try overriding it, make sure we still only have 1.
      config = create_test_dcm(false);
      let mut nobra_def = ProtocolDefinition::default();
      let mut serial_specifier = SerialSpecifier::default();
      serial_specifier.port = "COM1".to_owned();
      nobra_def.serial = Some(vec![serial_specifier]);
      config.add_protocol_definition("nobra", nobra_def);
      assert!(config.protocol_definitions().contains_key("nobra"));
      assert!(config
        .protocol_definitions()
        .get("nobra")
        .expect("Test, assuming infallible")
        .serial
        .as_ref()
        .is_some());
      assert_eq!(
        config
          .protocol_definitions()
          .get("nobra")
          .expect("Test, assuming infallible")
          .serial
          .as_ref()
          .expect("Test, assuming infallible")
          .len(),
        1
      );
      assert!(config
        .protocol_definitions()
        .get("nobra")
        .expect("Test, assuming infallible")
        .serial
        .as_ref()
        .expect("Test, assuming infallible")
        .iter()
        .any(|x| x.port == "COM1"));
    }
*/
// TODO Test invalid config load (not json)

// TODO Test calculation/change of Step Count via Step Range
