// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;

use buttplug_server_device_config::load_protocol_configs;
use tokio_test::assert_ok;

const BASE_CONFIG_JSON: &str = r#"
{
  "version": {
    "major": 5,
    "minor": 999
  },
  "protocols": {
    "aneros": {
      "communication": [
        {
          "btle": {
            "names": [
              "Massage Demo"
            ],
            "services": {
              "0000ff00-0000-1000-8000-00805f9b34fb": {
                "tx": "0000ff01-0000-1000-8000-00805f9b34fb"
              }
            }
          }
        }
      ],
      "defaults": {
        "features": [
          {
            "index": 0,
            "description": "Perineum Vibrator",
            "id": "a980bc1a-5554-4293-a75f-6d17bf25ebee",
            "output": {
              "vibrate": {
                "value": [
                  0,
                  127
                ]
              }
            }
          },
          {
            "index": 1,
            "description": "Internal Vibrator",
            "id": "811d7d6e-6a75-4925-943a-a06042223e3a",
            "output": {
              "vibrate": {
                "value": [
                  0,
                  127
                ]
              }
            }
          }
        ],
        "id": "f023f0f4-6629-469e-84c4-171ed4939f3d",
        "name": "Aneros Vivi"
      }
    }
  }
}
"#;

const BASE_VALID_VERSION_CONFIG_JSON: &str = r#"
{
  "version": {
    "major": 5,
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
    "major": 5,
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
  assert!(
    load_protocol_configs(
      &None,
      &Some(BASE_INVALID_VERSION_CONFIG_JSON.to_owned()),
      false
    )
    .is_err()
  );
}

#[tokio::test]
async fn test_basic_device_config() {
  assert!(load_protocol_configs(&Some(BASE_CONFIG_JSON.to_owned()), &None, false).is_ok());
}

#[tokio::test]
async fn test_valid_user_config() {
  let user_config_json = r#"
  {
    "version": {
      "major": 5,
      "minor": 999
    },
    "user_configs": {
      "devices": [
        {
          "identifier": {
            "address": "range-test",
            "protocol": "aneros",
            "identifier": "Massage Demo"
          },
          "config": {
            "name": "Aneros Vivi",
            "id": "66bf8a8e-bc5b-4074-9a9d-48892e1c74e2",
            "base_id": "f023f0f4-6629-469e-84c4-171ed4939f3d",
            "features": [
              {
                "id": "0b281420-8e58-43e6-ac35-2c3d49099255",
                "base_id": "a980bc1a-5554-4293-a75f-6d17bf25ebee",
                "output": {
                  "vibrate": {
                    "value": [
                      0,
                      64
                    ]
                  }
                }
              },
              {
                "id": "5ce69580-3f2a-4d60-8ad5-5bf664ce8e5f",
                "base_id": "811d7d6e-6a75-4925-943a-a06042223e3a"
              }
            ],
            "user_config": {
              "allow": false,
              "deny": false,
              "index": 0,
              "display_name": "Range Test"
            }
          }
        }
      ]
    }
  }"#;
  assert!(
    load_protocol_configs(
      &Some(BASE_CONFIG_JSON.to_owned()),
      &Some(user_config_json.to_owned()),
      false
    )
    .is_ok()
  );
}

#[tokio::test]
async fn test_invalid_step_range_device_config_wrong_range_length() {
  let user_config_json = r#"
  {
    "version": {
      "major": 5,
      "minor": 999
    },
    "user_configs": {
      "devices": [
        {
          "identifier": {
            "address": "range-test",
            "protocol": "aneros",
            "identifier": "Massage Demo"
          },
          "config": {
            "name": "Aneros Vivi",
            "id": "66bf8a8e-bc5b-4074-9a9d-48892e1c74e2",
            "base_id": "f023f0f4-6629-469e-84c4-171ed4939f3d",
            "features": [
              {
                "id": "0b281420-8e58-43e6-ac35-2c3d49099255",
                "base_id": "a980bc1a-5554-4293-a75f-6d17bf25ebee",
                "output": {
                  "vibrate": {
                    "value": [
                      10
                    ]
                  }
                }
              },
              {
                "id": "5ce69580-3f2a-4d60-8ad5-5bf664ce8e5f",
                "base_id": "811d7d6e-6a75-4925-943a-a06042223e3a"
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
  assert!(
    load_protocol_configs(
      &Some(BASE_CONFIG_JSON.to_owned()),
      &Some(user_config_json.to_owned()),
      false
    )
    .is_err()
  );
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
