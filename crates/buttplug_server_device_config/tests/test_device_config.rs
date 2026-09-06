// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_server_device_config::{
  ProtocolCommunicationSpecifier,
  SdlGamepadSpecifier,
  UserDeviceIdentifier,
  load_protocol_configs,
};
use test_case::test_case;

#[test]
fn test_sdl_gamepad_specifier_round_trip() {
  // JSON form, as it appears in the generated device config file.
  let from_json: ProtocolCommunicationSpecifier =
    serde_json::from_str(r#"{"sdl-gamepad": {"exists": true}}"#).unwrap();
  assert_eq!(
    from_json,
    ProtocolCommunicationSpecifier::SdlGamepad(SdlGamepadSpecifier::default())
  );
  let back_to_json = serde_json::to_string(&from_json).unwrap();
  assert_eq!(back_to_json, r#"{"sdl-gamepad":{"exists":true}}"#);

  // YAML form, as it appears in the protocol definition YAML files. The build
  // pipeline (see build.rs) parses YAML straight into serde_json::Value before
  // the config structs deserialize from it, so mirror that path here.
  let yaml_value: serde_json::Value =
    serde_yaml::from_str("- sdl-gamepad:\n    exists: true\n").unwrap();
  let from_yaml: ProtocolCommunicationSpecifier =
    serde_json::from_value(yaml_value[0].clone()).unwrap();
  assert_eq!(
    from_yaml,
    ProtocolCommunicationSpecifier::SdlGamepad(SdlGamepadSpecifier::default())
  );
}

#[test]
fn test_sdl_gamepad_protocol_in_generated_config() {
  let config = std::fs::read_to_string("build-config/buttplug-device-config-v5.json").unwrap();
  let json: serde_json::Value = serde_json::from_str(&config).unwrap();
  assert_eq!(json["version"]["major"], 5);
  let protocol = &json["protocols"]["sdl-gamepad"];
  assert_eq!(protocol["defaults"]["name"], "SDL Gamepad");
  let features = protocol["defaults"]["features"].as_array().unwrap();
  assert_eq!(features.len(), 2);
  for (i, feature) in features.iter().enumerate() {
    assert_eq!(feature["index"], i as u64);
    assert_eq!(feature["output"]["vibrate"]["value"][0], 0);
    assert_eq!(feature["output"]["vibrate"]["value"][1], 65535);
  }
  let communication = protocol["communication"][0]["sdl-gamepad"].clone();
  assert_eq!(communication["exists"], true);
}

#[test_case("version_only.json" ; "Version Only")]
#[test_case("base_aneros_protocol.json" ; "Aneros Protocol")]
#[test_case("base_tcode_protocol.json" ; "TCode Protocol")]
fn test_valid_base_config(test_file: &str) {
  load_protocol_configs(
    &Some(
      str::from_utf8(&std::fs::read(format!("tests/test_configs/{}", test_file)).unwrap())
        .unwrap()
        .to_owned(),
    ),
    &None,
    false,
  )
  .unwrap()
  .finish()
  .unwrap();
}

#[test_case("base_tcode_protocol.json", "user_tcode_protocol.json" ; "TCode Protocol")]
fn test_valid_user_config(base_config: &str, user_config: &str) {
  load_protocol_configs(
    &Some(
      str::from_utf8(&std::fs::read(format!("tests/test_configs/{}", base_config)).unwrap())
        .unwrap()
        .to_owned(),
    ),
    &Some(
      str::from_utf8(&std::fs::read(format!("tests/test_configs/{}", user_config)).unwrap())
        .unwrap()
        .to_owned(),
    ),
    false,
  )
  .unwrap()
  .finish()
  .unwrap();
}

#[test]
fn test_tcode_device_creation() {
  let dcm = load_protocol_configs(
    &Some(
      str::from_utf8(&std::fs::read("tests/test_configs/base_tcode_protocol.json").unwrap())
        .unwrap()
        .to_owned(),
    ),
    &Some(
      str::from_utf8(&std::fs::read("tests/test_configs/user_tcode_protocol.json").unwrap())
        .unwrap()
        .to_owned(),
    ),
    false,
  )
  .unwrap()
  .finish()
  .unwrap();
  let device = dcm
    .device_definition(&UserDeviceIdentifier::new("COM1", "tcode-v03", &None))
    .unwrap();
  assert_eq!(device.name(), "TCode v0.3 (Single Linear Axis)");
}
