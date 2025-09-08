use buttplug_server_device_config::{UserDeviceIdentifier, load_protocol_configs};
use test_case::test_case;

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
