// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;
extern crate buttplug;

use buttplug::{
  server::ButtplugServerBuilder,
  util::async_manager
};

const BASE_CONFIG_JSON: &str = r#"
{
  "version": 63,
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
            "VibrateCmd": {
              "FeatureCount": 1,
              "StepCount": [
                100
              ]
            }
          }
        }
      ]
    }
  }
}
"#;

#[cfg(feature = "server")]
#[test]
fn test_basic_device_config() {
  async_manager::block_on(async move {
    assert!(ButtplugServerBuilder::default().device_configuration_json(Some(BASE_CONFIG_JSON.to_owned())).finish().is_ok());
  });
}


#[cfg(feature = "server")]
#[test]
fn test_valid_step_range() {
  //tracing_subscriber::fmt::init();
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
  async_manager::block_on(async move {
    assert!(ButtplugServerBuilder::default().device_configuration_json(Some(BASE_CONFIG_JSON.to_owned())).user_device_configuration_json(Some(user_config_json.to_owned())).finish().is_ok());
  });
}

#[cfg(feature = "server")]
#[test]
fn test_invalid_step_range_device_config_wrong_range_length() {
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
  async_manager::block_on(async move {
    assert!(ButtplugServerBuilder::default().device_configuration_json(Some(BASE_CONFIG_JSON.to_owned())).user_device_configuration_json(Some(user_config_json.to_owned())).finish().is_err());
  });
}

#[cfg(feature = "server")]
#[test]
fn test_invalid_step_range_device_config_wrong_order() {
  tracing_subscriber::fmt::init();
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
  async_manager::block_on(async move {
    assert!(ButtplugServerBuilder::default().device_configuration_json(Some(BASE_CONFIG_JSON.to_owned())).user_device_configuration_json(Some(user_config_json.to_owned())).finish().is_ok());
    assert!(ButtplugServerBuilder::default().device_configuration_json(Some(BASE_CONFIG_JSON.to_owned())).user_device_configuration_json(Some(user_config_json.to_owned())).finish().is_ok());
  });
}
