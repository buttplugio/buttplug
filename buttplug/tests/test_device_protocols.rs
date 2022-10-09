mod util;
use buttplug::util::async_manager;
use test_case::test_case;
use util::DeviceTestCase;

async fn load_test_case(test_file: &str) -> DeviceTestCase {
  // Load the file list from the test cases directory
  let test_file_path =
    std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").expect("Should have manifest path"))
      .join("tests")
      .join("util")
      .join("device_test")
      .join("device_test_case")
      .join(test_file);
  // Given the test case object, run the test across all client versions.
  let yaml_test_case = std::fs::read_to_string(&test_file_path)
    .unwrap_or_else(|_| panic!("Cannot read file {:?}", test_file_path));
  serde_yaml::from_str(&yaml_test_case).expect("Could not parse yaml for file.")
}

#[test_case("test_aneros_protocol.yaml" ; "Aneros Protocol")]
#[test_case("test_ankni_protocol.yaml" ; "Ankni Protocol")]
#[test_case("test_cachito_protocol.yaml" ; "Cachito Protocol")]
#[test_case("test_fredorch_protocol.yaml" ; "Fredorch Protocol")]
#[test_case("test_hismith_thrusting_cup.yaml" ; "Hismith Protocol - Thrusting Cup")]
#[test_case("test_lovense_single_vibrator.yaml" ; "Lovense Protocol - Single Vibrator Device")]
#[test_case("test_lovense_max.yaml" ; "Lovense Protocol - Lovense Max (Vibrate/Constrict)")]
#[test_case("test_lovense_nora.yaml" ; "Lovense Protocol - Lovense Nora (Vibrate/Rotate)")]
#[test_case("test_lovense_ridge.yaml" ; "Lovense Protocol - Lovense Ridge (Oscillate)")]
#[test_case("test_lovense_battery.yaml" ; "Lovense Protocol - Lovense Battery (Default Devices)")]
#[test_case("test_lovense_battery_non_default.yaml" ; "Lovense Protocol - Lovense Battery (Non-Default Devices)")]
#[test_case("test_lovense_ridge_user_config.yaml" ; "Lovense Protocol - Lovense Ridge (User Config)")]
#[test_case("test_user_config_display_name.yaml" ; "User Config Display Name")]
#[test_case("test_satisfyer_single_vibrator.yaml" ; "Satisfyer Protocol - Single Vibrator")]
#[test_case("test_satisfyer_dual_vibrator.yaml" ; "Satisfyer Protocol - Dual Vibrator")]
#[test_case("test_mysteryvibe.yaml" ; "Mysteryvibe Protocol")]
#[test_case("test_meese_protocol.yaml" ; "Meese Protocol")]
#[test_case("test_mizzzee_protocol.yaml" ; "Mizz Zee Protocol")]
#[test_case("test_mizzzee_v2_protocol.yaml" ; "Mizz Zee v2 Protocol")]
#[test_case("test_vorze_ufo.yaml" ; "Vorze Protocol - UFO")]
#[test_case("test_vorze_ufo.yaml" ; "Vorze Protocol - UFO TW")]
#[test_case("test_wevibe_4plus.yaml" ; "WeVibe Protocol (Legacy) - 4 Plus")]
#[test_case("test_wevibe_pivot.yaml" ; "WeVibe Protocol (Legacy) - Pivot")]
#[test_case("test_wevibe_vector.yaml" ; "WeVibe Protocol (8bit) - Vector")]
#[test_case("test_wevibe_moxie.yaml" ; "WeVibe Protocol (8bit) - Moxie")]
#[test_case("test_wevibe_chorus.yaml" ; "WeVibe Protocol (Chorus) - Chorus")]
#[test_case("test_nobra_protocol.yaml" ; "Nobra Protocol")]
#[test_case("test_lovehoney_desire_prostate.yaml" ; "Lovehoney Desire Protocol - Prostate Vibe")]
#[test_case("test_lovehoney_desire_egg.yaml" ; "Lovehoney Desire Protocol - Love Egg")]
#[test_case("test_wetoy_protocol.yaml" ; "WeToy Protocol")]
#[test_case("test_pink_punch_protocol.yaml" ; "Pink Punch Protocol")]
#[test_case("test_sakuraneko_protocol.yaml" ; "Sakuraneko Protocol")]
#[test_case("test_synchro_protocol.yaml" ; "Synchro Protocol")]
#[test_case("test_lelo_tianiharmony.yaml" ; "Lelo Harmony Protocol - Tiani Harmony")]
#[test_case("test_lelo_idawave.yaml" ; "Lelo Harmony Protocol - Ida Wave")]
fn test_device_protocols_embedded_v3(test_file: &str) {
  //tracing_subscriber::fmt::init();
  async_manager::block_on(async {
    util::device_test::client::client_v3::run_embedded_test_case(&load_test_case(test_file).await)
      .await;
  });
}

#[test_case("test_aneros_protocol.yaml" ; "Aneros Protocol")]
#[test_case("test_ankni_protocol.yaml" ; "Ankni Protocol")]
#[test_case("test_cachito_protocol.yaml" ; "Cachito Protocol")]
#[test_case("test_fredorch_protocol.yaml" ; "Fredorch Protocol")]
#[test_case("test_hismith_thrusting_cup.yaml" ; "Hismith Protocol - Thrusting Cup")]
#[test_case("test_lovense_single_vibrator.yaml" ; "Lovense Protocol - Single Vibrator Device")]
#[test_case("test_lovense_max.yaml" ; "Lovense Protocol - Lovense Max (Vibrate/Constrict)")]
#[test_case("test_lovense_nora.yaml" ; "Lovense Protocol - Lovense Nora (Vibrate/Rotate)")]
#[test_case("test_lovense_ridge.yaml" ; "Lovense Protocol - Lovense Ridge (Oscillate)")]
#[test_case("test_lovense_battery.yaml" ; "Lovense Protocol - Lovense Battery (Default Devices)")]
#[test_case("test_lovense_battery_non_default.yaml" ; "Lovense Protocol - Lovense Battery (Non-Default Devices)")]
#[test_case("test_lovense_ridge_user_config.yaml" ; "Lovense Protocol - Lovense Ridge (User Config)")]
#[test_case("test_user_config_display_name.yaml" ; "User Config Display Name")]
#[test_case("test_satisfyer_single_vibrator.yaml" ; "Satisfyer Protocol - Single Vibrator")]
#[test_case("test_satisfyer_dual_vibrator.yaml" ; "Satisfyer Protocol - Dual Vibrator")]
#[test_case("test_satisfyer_triple_vibrator.yaml" ; "Satisfyer Protocol - Triple Vibrator")]
#[test_case("test_mysteryvibe.yaml" ; "Mysteryvibe Protocol")]
#[test_case("test_meese_protocol.yaml" ; "Meese Protocol")]
#[test_case("test_mizzzee_protocol.yaml" ; "Mizz Zee Protocol")]
#[test_case("test_mizzzee_v2_protocol.yaml" ; "Mizz Zee v2 Protocol")]
#[test_case("test_vorze_ufo.yaml" ; "Vorze Protocol - UFO")]
#[test_case("test_vorze_ufo.yaml" ; "Vorze Protocol - UFO TW")]
#[test_case("test_wevibe_4plus.yaml" ; "WeVibe Protocol (Legacy) - 4 Plus")]
#[test_case("test_wevibe_pivot.yaml" ; "WeVibe Protocol (Legacy) - Pivot")]
#[test_case("test_wevibe_vector.yaml" ; "WeVibe Protocol (8bit) - Vector")]
#[test_case("test_wevibe_moxie.yaml" ; "WeVibe Protocol (8bit) - Moxie")]
#[test_case("test_wevibe_chorus.yaml" ; "WeVibe Protocol (Chorus) - Chorus")]
#[test_case("test_nobra_protocol.yaml" ; "Nobra Protocol")]
#[test_case("test_lovehoney_desire_prostate.yaml" ; "Lovehoney Desire Protocol - Prostate Vibe")]
#[test_case("test_lovehoney_desire_egg.yaml" ; "Lovehoney Desire Protocol - Love Egg")]
#[test_case("test_wetoy_protocol.yaml" ; "WeToy Protocol")]
#[test_case("test_pink_punch_protocol.yaml" ; "Pink Punch Protocol")]
#[test_case("test_sakuraneko_protocol.yaml" ; "Sakuraneko Protocol")]
#[test_case("test_synchro_protocol.yaml" ; "Synchro Protocol")]
#[test_case("test_lelo_tianiharmony.yaml" ; "Lelo Harmony Protocol - Tiani Harmony")]
#[test_case("test_lelo_idawave.yaml" ; "Lelo Harmony Protocol - Ida Wave")]
fn test_device_protocols_json_v3(test_file: &str) {
  //tracing_subscriber::fmt::init();
  async_manager::block_on(async {
    util::device_test::client::client_v3::run_json_test_case(&load_test_case(test_file).await)
      .await;
  });
}

#[test_case("test_aneros_protocol.yaml" ; "Aneros Protocol")]
#[test_case("test_ankni_protocol.yaml" ; "Ankni Protocol")]
#[test_case("test_cachito_protocol.yaml" ; "Cachito Protocol")]
#[test_case("test_fredorch_protocol.yaml" ; "Fredorch Protocol")]
#[test_case("test_lovense_single_vibrator.yaml" ; "Lovense Protocol - Single Vibrator Device")]
#[test_case("test_lovense_nora.yaml" ; "Lovense Protocol - Lovense Nora (Vibrate/Rotate)")]
#[test_case("test_lovense_battery.yaml" ; "Lovense Protocol - Lovense Battery (Default Devices)")]
#[test_case("test_lovense_battery_non_default.yaml" ; "Lovense Protocol - Lovense Battery (Non-Default Devices)")]
#[test_case("test_satisfyer_single_vibrator.yaml" ; "Satisfyer Protocol - Single Vibrator")]
#[test_case("test_satisfyer_dual_vibrator.yaml" ; "Satisfyer Protocol - Dual Vibrator")]
#[test_case("test_satisfyer_triple_vibrator.yaml" ; "Satisfyer Protocol - Triple Vibrator")]
#[test_case("test_mysteryvibe.yaml" ; "Mysteryvibe Protocol")]
#[test_case("test_meese_protocol.yaml" ; "Meese Protocol")]
#[test_case("test_mizzzee_protocol.yaml" ; "Mizz Zee Protocol")]
#[test_case("test_mizzzee_v2_protocol.yaml" ; "Mizz Zee v2 Protocol")]
#[test_case("test_vorze_ufo.yaml" ; "Vorze Protocol - UFO")]
#[test_case("test_vorze_ufo.yaml" ; "Vorze Protocol - UFO TW")]
#[test_case("test_wevibe_4plus.yaml" ; "WeVibe Protocol (Legacy) - 4 Plus")]
#[test_case("test_wevibe_pivot.yaml" ; "WeVibe Protocol (Legacy) - Pivot")]
#[test_case("test_wevibe_vector.yaml" ; "WeVibe Protocol (8bit) - Vector")]
#[test_case("test_wevibe_moxie.yaml" ; "WeVibe Protocol (8bit) - Moxie")]
#[test_case("test_wevibe_chorus.yaml" ; "WeVibe Protocol (Chorus) - Chorus")]
#[test_case("test_nobra_protocol.yaml" ; "Nobra Protocol")]
#[test_case("test_lovehoney_desire_prostate.yaml" ; "Lovehoney Desire Protocol - Prostate Vibe")]
#[test_case("test_lovehoney_desire_egg.yaml" ; "Lovehoney Desire Protocol - Love Egg")]
#[test_case("test_wetoy_protocol.yaml" ; "WeToy Protocol")]
#[test_case("test_pink_punch_protocol.yaml" ; "Pink Punch Protocol")]
#[test_case("test_sakuraneko_protocol.yaml" ; "Sakuraneko Protocol")]
#[test_case("test_synchro_protocol.yaml" ; "Synchro Protocol")]
#[test_case("test_lelo_tianiharmony.yaml" ; "Lelo Harmony Protocol - Tiani Harmony")]
fn test_device_protocols_embedded_v2(test_file: &str) {
  async_manager::block_on(async {
    util::device_test::client::client_v2::run_embedded_test_case(&load_test_case(test_file).await)
      .await;
  });
}

#[test_case("test_aneros_protocol.yaml" ; "Aneros Protocol")]
#[test_case("test_ankni_protocol.yaml" ; "Ankni Protocol")]
#[test_case("test_cachito_protocol.yaml" ; "Cachito Protocol")]
#[test_case("test_fredorch_protocol.yaml" ; "Fredorch Protocol")]
#[test_case("test_lovense_single_vibrator.yaml" ; "Lovense Protocol - Single Vibrator Device")]
#[test_case("test_lovense_nora.yaml" ; "Lovense Protocol - Lovense Nora (Vibrate/Rotate)")]
#[test_case("test_lovense_battery.yaml" ; "Lovense Protocol - Lovense Battery (Default Devices)")]
#[test_case("test_lovense_battery_non_default.yaml" ; "Lovense Protocol - Lovense Battery (Non-Default Devices)")]
#[test_case("test_satisfyer_single_vibrator.yaml" ; "Satisfyer Protocol - Single Vibrator")]
#[test_case("test_satisfyer_dual_vibrator.yaml" ; "Satisfyer Protocol - Dual Vibrator")]
#[test_case("test_satisfyer_triple_vibrator.yaml" ; "Satisfyer Protocol - Triple Vibrator")]
#[test_case("test_mysteryvibe.yaml" ; "Mysteryvibe Protocol")]
#[test_case("test_meese_protocol.yaml" ; "Meese Protocol")]
#[test_case("test_mizzzee_protocol.yaml" ; "Mizz Zee Protocol")]
#[test_case("test_mizzzee_v2_protocol.yaml" ; "Mizz Zee v2 Protocol")]
#[test_case("test_vorze_ufo.yaml" ; "Vorze Protocol - UFO")]
#[test_case("test_vorze_ufo.yaml" ; "Vorze Protocol - UFO TW")]
#[test_case("test_wevibe_4plus.yaml" ; "WeVibe Protocol (Legacy) - 4 Plus")]
#[test_case("test_wevibe_pivot.yaml" ; "WeVibe Protocol (Legacy) - Pivot")]
#[test_case("test_wevibe_vector.yaml" ; "WeVibe Protocol (8bit) - Vector")]
#[test_case("test_wevibe_moxie.yaml" ; "WeVibe Protocol (8bit) - Moxie")]
#[test_case("test_wevibe_chorus.yaml" ; "WeVibe Protocol (Chorus) - Chorus")]
#[test_case("test_nobra_protocol.yaml" ; "Nobra Protocol")]
#[test_case("test_lovehoney_desire_prostate.yaml" ; "Lovehoney Desire Protocol - Prostate Vibe")]
#[test_case("test_lovehoney_desire_egg.yaml" ; "Lovehoney Desire Protocol - Love Egg")]
#[test_case("test_wetoy_protocol.yaml" ; "WeToy Protocol")]
#[test_case("test_pink_punch_protocol.yaml" ; "Pink Punch Protocol")]
#[test_case("test_sakuraneko_protocol.yaml" ; "Sakuraneko Protocol")]
#[test_case("test_synchro_protocol.yaml" ; "Synchro Protocol")]
#[test_case("test_lelo_tianiharmony.yaml" ; "Lelo Harmony Protocol - Tiani Harmony")]
fn test_device_protocols_json_v2(test_file: &str) {
  async_manager::block_on(async {
    util::device_test::client::client_v2::run_json_test_case(&load_test_case(test_file).await)
      .await;
  });
}
