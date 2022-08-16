mod util;
use buttplug::{
  util::async_manager,
};
use test_case::test_case;


#[test_case("test_aneros_protocol.yaml" ; "Aneros Protocol")]
#[test_case("test_ankni_protocol.yaml" ; "Ankni Protocol")]
#[test_case("test_cachito_protocol.yaml" ; "Cachito Protocol")]
#[test_case("test_fredorch_protocol.yaml" ; "Fredorch Protocol")]
#[test_case("test_lovense_single_vibrator.yaml" ; "Lovense Protocol - Single Vibrator Device")]
#[test_case("test_lovense_max.yaml" ; "Lovense Protocol - Lovense Max (Vibrate/Constrict)")]
#[test_case("test_lovense_nora.yaml" ; "Lovense Protocol - Lovense Nora (Vibrate/Rotate)")]
#[test_case("test_lovense_ridge.yaml" ; "Lovense Protocol - Lovense Ridge (Oscillate)")]
#[test_case("test_lovense_battery.yaml" ; "Lovense Protocol - Lovense Battery (Default Devices)")]
#[test_case("test_lovense_battery_non_default.yaml" ; "Lovense Protocol - Lovense Battery (Non-Default Devices)")]
#[test_case("test_lovense_ridge_user_config.yaml" ; "Lovense Protocol - Lovense Ridge (User Config)")]
fn test_device_protocols_v3(test_file: &str) {
  async_manager::block_on(async {
    // Load the file list from the test cases directory
    let test_file_path = std::path::Path::new(
      &std::env::var("CARGO_MANIFEST_DIR").expect("Should have manifest path"),
    )
    .join("tests")
    .join("util")
    .join("device_test")
    .join("device_test_case")
    .join(test_file);
    // Given the test case object, run the test across all client versions.
    let yaml_test_case = std::fs::read_to_string(&test_file_path)
      .expect(&format!("Cannot read file {:?}", test_file_path));
    let test_case = serde_yaml::from_str(&yaml_test_case).expect("Could not parse yaml for file.");
    util::run_v3_device_test_case(&test_case).await;
  });
}
