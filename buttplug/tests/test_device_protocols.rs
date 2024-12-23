mod util;
//use buttplug::util::async_manager;
use test_case::test_case;
use util::device_test::DeviceTestCase;

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
#[test_case("test_ankni_protocol_no_handshake.yaml" ; "Ankni Protocol - No Handshake")]
#[test_case("test_cachito_protocol.yaml" ; "Cachito Protocol")]
#[test_case("test_fredorch_protocol.yaml" ; "Fredorch Protocol")]
#[test_case("test_hismith_auxfun_box.yaml" ; "Hismith Mini Protocol - Auxfun Box")]
#[test_case("test_hismith_v4.yaml" ; "Hismith Mini Protocol - Hismith v4")]
#[test_case("test_hismith_sinloli.yaml" ; "Hismith Mini Protocol - Sinloli")]
#[test_case("test_hismith_thrusting_cup.yaml" ; "Hismith Protocol - Thrusting Cup")]
#[test_case("test_hismith_wildolo.yaml" ; "Hismith Protocol - Wildolo")]
#[test_case("test_lovense_single_vibrator.yaml" ; "Lovense Protocol - Single Vibrator Device")]
#[test_case("test_lovense_max.yaml" ; "Lovense Protocol - Lovense Max (Vibrate/Constrict)")]
#[test_case("test_lovense_nora.yaml" ; "Lovense Protocol - Lovense Nora (Vibrate/Rotate)")]
#[test_case("test_lovense_ridge.yaml" ; "Lovense Protocol - Lovense Ridge (Oscillate)")]
#[test_case("test_lovense_battery.yaml" ; "Lovense Protocol - Lovense Battery (Default Devices)")]
#[test_case("test_lovense_battery_non_default.yaml" ; "Lovense Protocol - Lovense Battery (Non-Default Devices)")]
#[test_case("test_lovense_ridge_user_config.yaml" ; "Lovense Protocol - Lovense Ridge (User Config)")]
#[test_case("test_lovense_flexer_fw2.yaml" ; "Lovense Protocol - Flexer FW2")]
#[test_case("test_lovense_flexer_fw3.yaml" ; "Lovense Protocol - Flexer FW3")]
#[test_case("test_lovense_edge.yaml" ; "Lovense Protocol - Edge")]
#[test_case("test_lovense_osci3.yaml" ; "Lovense Protocol - Osci3")]
#[test_case("test_user_config_display_name.yaml" ; "User Config Display Name")]
#[test_case("test_satisfyer_single_vibrator.yaml" ; "Satisfyer Protocol - Single Vibrator")]
#[test_case("test_satisfyer_dual_vibrator.yaml" ; "Satisfyer Protocol - Dual Vibrator")]
#[test_case("test_mysteryvibe.yaml" ; "Mysteryvibe Protocol")]
#[test_case("test_meese_protocol.yaml" ; "Meese Protocol")]
#[test_case("test_mizzzee_protocol.yaml" ; "Mizz Zee Protocol")]
#[test_case("test_mizzzee_v2_protocol.yaml" ; "Mizz Zee v2 Protocol")]
#[test_case("test_mizzzee_v3_protocol.yaml" ; "Mizz Zee v3 Protocol")]
#[test_case("test_vorze_ufo.yaml" ; "Vorze Protocol - UFO")]
#[test_case("test_vorze_ufo_tw.yaml" ; "Vorze Protocol - UFO TW")]
#[test_case("test_vorze_cyclone.yaml" ; "Vorze Protocol - Cyclone")]
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
#[test_case("test_lelo_f1sv1.yaml" ; "Lelo F1s V1 Protocol")]
#[test_case("test_lelo_f1sv2.yaml" ; "Lelo F1s V2 Protocol")]
#[test_case("test_lelo_idawave.yaml" ; "Lelo Harmony Protocol - Ida Wave")]
#[test_case("test_magic_motion_1_magic_cell.yaml" ; "MagicMotion Protocol 1 - Magic Cell")]
#[test_case("test_magic_motion_2_eidolon.yaml" ; "MagicMotion Protocol 2 - Eidolon")]
#[test_case("test_magic_motion_2_equinox.yaml" ; "MagicMotion Protocol 2 - Equinox")]
#[test_case("test_magic_motion_3_krush.yaml" ; "MagicMotion Protocol 3 - Krush")]
#[test_case("test_magic_motion_4_bobi.yaml" ; "MagicMotion Protocol 4 - Bobi")]
#[test_case("test_magic_motion_4_nyx.yaml" ; "MagicMotion Protocol 4 - Nyx")]
#[test_case("test_hgod_protocol.yaml" ; "Hgod Protocol")]
#[test_case("test_tryfun_protocol.yaml" ; "TryFun Protocol")]
#[test_case("test_tryfun_surge.yaml" ; "TryFun Protocol - Surge Pro")]
#[test_case("test_metaxsire_rex.yaml" ; "metaXsire Protocol - Rex")]
#[test_case("test_metaxsire_olis.yaml" ; "metaXsire Protocol - Olis")]
#[test_case("test_metaxsire_cali.yaml" ; "metaXsire Protocol - Cali")]
#[test_case("test_metaxsire_nolan.yaml" ; "metaXsire Protocol v2 - Nolan")]
#[test_case("test_cowgirl_protocol.yaml" ; "The Cowgirl Protocol")]
#[test_case("test_galaku_nebula.yaml" ; "Galaku Pump Protocol - Nebula")]
#[test_case("test_galaku.yaml" ; "Galaku Protocol")]
#[test_case("test_xibao_protocol.yaml" ; "Xibao Protocol")]
#[test_case("test_sensee_protocol.yaml" ; "Sensee Diandou Protocol - Rabbit")]
#[test_case("test_sensee_capsule.yaml" ; "Sensee Capsule Protocol")]
#[test_case("test_svakom_pulse.yaml" ; "Svakom Pulse Protocol - Pulse Lite Neo")]
#[test_case("test_svakom_ella.yaml" ; "Svakom V1 Protocol - Ella")]
#[test_case("test_svakom_vivianna.yaml" ; "Svakom V2 Protocol - Vivianna")]
#[test_case("test_svakom_theodore.yaml" ; "Svakom V3 Protocol - Theodore")]
#[test_case("test_svakom_alex.yaml" ; "Svakom Alex Neo")]
#[test_case("test_svakom_alex_v2.yaml" ; "Svakom Alex Neo 2")]
#[test_case("test_svakom_iker.yaml" ; "Svakom Iker")]
#[test_case("test_svakom_barnard.yaml" ; "Svakom (Fantasy Cup) Barnard")]
#[test_case("test_svakom_mora_neo.yaml" ; "Svakom Mora Neo")]
#[test_case("test_fox_protocol.yaml" ; "Fox Protocol")]
#[test_case("test_sakuraneko_koikoi.yaml" ; "Sakuraneko Protocol - Koikoi")]
#[test_case("test_xiuxiuda_protocol.yaml" ; "Xiuxiuda Protocol")]
#[test_case("test_longlosttouch_protocol.yaml" ; "LongLostTouch Protocol")]
#[test_case("test_adrienlastic_protocol.yaml" ; "Adrien Lastic Protocol")]
#[test_case("test_foreo_protocol.yaml" ; "Foreo Protocol")]
#[test_case("test_joyhub_petalwish.yaml" ; "JoyHub Protocol - Petalwish")]
#[test_case("test_joyhub_petalwish_compat.yaml" ; "JoyHub Protocol - Petalwish Compat")]
#[test_case("test_joyhub_roselin.yaml" ; "JoyHub Protocol - RoseLin")]
#[test_case("test_joyhub_moonhorn.yaml" ; "JoyHub Protocol - Moonhorn")]
#[test_case("test_itoys_protocol.yaml" ; "iToys Protocol")]
#[test_case("test_leten_protocol.yaml" ; "Leten Protocol")]
#[test_case("test_motorbunny_protocol.yaml" ; "Motorbunny Protocol")]
#[test_case("test_activejoy_protocol.yaml" ; "ActiveJoy Protocol")]
#[test_case("test_cupido_protocol.yaml" ; "Cupido Protocol")]
//#[test_case("test_cowgirl_cone_protocol.yaml" ; "The Cowgirl Cone Protocol")]
#[test_case("test_amorelie_joy_protocol.yaml" ; "Amorelie Joy Protocol")]
#[test_case("test_svakom_sam2.yaml" ; "Svakom Sam Neo 2 Pro")]
#[test_case("test_feelingso.yaml" ; "FeelingSo Protocol")]
#[test_case("test_deepsire.yaml" ; "DeepSire Protocol")]
#[test_case("test_xuanhuan_protocol.yaml" ; "Xuanhuan Protocol")]
#[test_case("test_tcode_linear_and_vibrate.yaml" ; "TCode (Linear + Vibrate)")]
#[test_case("test_serveu_protocol.yaml" ; "ServeU")]
#[test_case("test_kiiroo_prowand.yaml" ; "Kiiroo ProWand Protocol")]
#[test_case("test_fleshy_thrust_protocol.yaml" ; "Fleshy Thrust Sync Protocol")]
#[tokio::test]
async fn test_device_protocols_embedded_v3(test_file: &str) {
  //tracing_subscriber::fmt::init();
  //error!("RUNNING TEST CASE");
  util::device_test::client::client_v3::run_embedded_test_case(&load_test_case(test_file).await)
    .await;
}

#[test_case("test_aneros_protocol.yaml" ; "Aneros Protocol")]
#[test_case("test_ankni_protocol.yaml" ; "Ankni Protocol")]
#[test_case("test_ankni_protocol_no_handshake.yaml" ; "Ankni Protocol - No Handshake")]
#[test_case("test_cachito_protocol.yaml" ; "Cachito Protocol")]
#[test_case("test_fredorch_protocol.yaml" ; "Fredorch Protocol")]
#[test_case("test_hismith_auxfun_box.yaml" ; "Hismith Mini Protocol - Auxfun Box")]
#[test_case("test_hismith_v4.yaml" ; "Hismith Mini Protocol - Hismith v4")]
#[test_case("test_hismith_sinloli.yaml" ; "Hismith Mini Protocol - Sinloli")]
#[test_case("test_hismith_thrusting_cup.yaml" ; "Hismith Protocol - Thrusting Cup")]
#[test_case("test_hismith_wildolo.yaml" ; "Hismith Protocol - Wildolo")]
#[test_case("test_lovense_single_vibrator.yaml" ; "Lovense Protocol - Single Vibrator Device")]
#[test_case("test_lovense_max.yaml" ; "Lovense Protocol - Lovense Max (Vibrate/Constrict)")]
#[test_case("test_lovense_nora.yaml" ; "Lovense Protocol - Lovense Nora (Vibrate/Rotate)")]
#[test_case("test_lovense_ridge.yaml" ; "Lovense Protocol - Lovense Ridge (Oscillate)")]
#[test_case("test_lovense_battery.yaml" ; "Lovense Protocol - Lovense Battery (Default Devices)")]
#[test_case("test_lovense_battery_non_default.yaml" ; "Lovense Protocol - Lovense Battery (Non-Default Devices)")]
#[test_case("test_lovense_ridge_user_config.yaml" ; "Lovense Protocol - Lovense Ridge (User Config)")]
#[test_case("test_lovense_flexer_fw2.yaml" ; "Lovense Protocol - Flexer FW2")]
#[test_case("test_lovense_flexer_fw3.yaml" ; "Lovense Protocol - Flexer FW3")]
#[test_case("test_lovense_edge.yaml" ; "Lovense Protocol - Edge")]
#[test_case("test_lovense_osci3.yaml" ; "Lovense Protocol - Osci3")]
#[test_case("test_user_config_display_name.yaml" ; "User Config Display Name")]
#[test_case("test_satisfyer_single_vibrator.yaml" ; "Satisfyer Protocol - Single Vibrator")]
#[test_case("test_satisfyer_dual_vibrator.yaml" ; "Satisfyer Protocol - Dual Vibrator")]
#[test_case("test_satisfyer_triple_vibrator.yaml" ; "Satisfyer Protocol - Triple Vibrator")]
#[test_case("test_mysteryvibe.yaml" ; "Mysteryvibe Protocol")]
#[test_case("test_meese_protocol.yaml" ; "Meese Protocol")]
#[test_case("test_mizzzee_protocol.yaml" ; "Mizz Zee Protocol")]
#[test_case("test_mizzzee_v2_protocol.yaml" ; "Mizz Zee v2 Protocol")]
#[test_case("test_mizzzee_v3_protocol.yaml" ; "Mizz Zee v3 Protocol")]
#[test_case("test_vorze_ufo.yaml" ; "Vorze Protocol - UFO")]
#[test_case("test_vorze_ufo_tw.yaml" ; "Vorze Protocol - UFO TW")]
#[test_case("test_vorze_cyclone.yaml" ; "Vorze Protocol - Cyclone")]
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
#[test_case("test_lelo_f1sv1.yaml" ; "Lelo F1s V1 Protocol")]
#[test_case("test_lelo_f1sv2.yaml" ; "Lelo F1s V2 Protocol")]
#[test_case("test_lelo_idawave.yaml" ; "Lelo Harmony Protocol - Ida Wave")]
#[test_case("test_magic_motion_1_magic_cell.yaml" ; "MagicMotion Protocol 1 - Magic Cell")]
#[test_case("test_magic_motion_2_eidolon.yaml" ; "MagicMotion Protocol 2 - Eidolon")]
#[test_case("test_magic_motion_2_equinox.yaml" ; "MagicMotion Protocol 2 - Equinox")]
#[test_case("test_magic_motion_3_krush.yaml" ; "MagicMotion Protocol 3 - Krush")]
#[test_case("test_magic_motion_4_bobi.yaml" ; "MagicMotion Protocol 4 - Bobi")]
#[test_case("test_magic_motion_4_nyx.yaml" ; "MagicMotion Protocol 4 - Nyx")]
#[test_case("test_hgod_protocol.yaml" ; "Hgod Protocol")]
#[test_case("test_tryfun_protocol.yaml" ; "TryFun Protocol")]
#[test_case("test_tryfun_surge.yaml" ; "TryFun Protocol - Surge Pro")]
#[test_case("test_metaxsire_rex.yaml" ; "metaXsire Protocol - Rex")]
#[test_case("test_metaxsire_olis.yaml" ; "metaXsire Protocol - Olis")]
#[test_case("test_metaxsire_cali.yaml" ; "metaXsire Protocol - Cali")]
#[test_case("test_metaxsire_nolan.yaml" ; "metaXsire Protocol v2 - Nolan")]
#[test_case("test_cowgirl_protocol.yaml" ; "The Cowgirl Protocol")]
#[test_case("test_galaku_nebula.yaml" ; "Galaku Pump Protocol - Nebula")]
#[test_case("test_galaku.yaml" ; "Galaku Protocol")]
#[test_case("test_xibao_protocol.yaml" ; "Xibao Protocol")]
#[test_case("test_sensee_protocol.yaml" ; "Sensee Diandou Protocol - Rabbit")]
#[test_case("test_sensee_capsule.yaml" ; "Sensee Capsule Protocol")]
#[test_case("test_svakom_pulse.yaml" ; "Svakom Pulse Protocol - Pulse Lite Neo")]
#[test_case("test_svakom_ella.yaml" ; "Svakom V1 Protocol - Ella")]
#[test_case("test_svakom_vivianna.yaml" ; "Svakom V2 Protocol - Vivianna")]
#[test_case("test_svakom_theodore.yaml" ; "Svakom V3 Protocol - Theodore")]
#[test_case("test_svakom_alex.yaml" ; "Svakom Alex Neo")]
#[test_case("test_svakom_alex_v2.yaml" ; "Svakom Alex Neo 2")]
#[test_case("test_svakom_barnard.yaml" ; "Svakom (Fantasy Cup) Barnard")]
#[test_case("test_svakom_mora_neo.yaml" ; "Svakom Mora Neo")]
#[test_case("test_svakom_iker.yaml" ; "Svakom Iker")]
#[test_case("test_fox_protocol.yaml" ; "Fox Protocol")]
#[test_case("test_sakuraneko_koikoi.yaml" ; "Sakuraneko Protocol - Koikoi")]
#[test_case("test_xiuxiuda_protocol.yaml" ; "Xiuxiuda Protocol")]
#[test_case("test_adrienlastic_protocol.yaml" ; "Adrien Lastic Protocol")]
#[test_case("test_foreo_protocol.yaml" ; "Foreo Protocol")]
#[test_case("test_joyhub_petalwish.yaml" ; "JoyHub Protocol - Petalwish")]
#[test_case("test_joyhub_petalwish_compat.yaml" ; "JoyHub Protocol - Petalwish Compat")]
#[test_case("test_joyhub_moonhorn.yaml" ; "JoyHub Protocol - Moonhorn")]
#[test_case("test_joyhub_roselin.yaml" ; "JoyHub Protocol - RoseLin")]
#[test_case("test_itoys_protocol.yaml" ; "iToys Protocol")]
#[test_case("test_leten_protocol.yaml" ; "Leten Protocol")]
#[test_case("test_motorbunny_protocol.yaml" ; "Motorbunny Protocol")]
#[test_case("test_activejoy_protocol.yaml" ; "ActiveJoy Protocol")]
#[test_case("test_cupido_protocol.yaml" ; "Cupido Protocol")]
//#[test_case("test_cowgirl_cone_protocol.yaml" ; "The Cowgirl Cone Protocol")]
#[test_case("test_amorelie_joy_protocol.yaml" ; "Amorelie Joy Protocol")]
#[test_case("test_svakom_sam2.yaml" ; "Svakom Sam Neo 2 Pro")]
#[test_case("test_feelingso.yaml" ; "FeelingSo Protocol")]
#[test_case("test_deepsire.yaml" ; "DeepSire Protocol")]
#[test_case("test_xuanhuan_protocol.yaml" ; "Xuanhuan Protocol")]
#[test_case("test_tcode_linear_and_vibrate.yaml" ; "TCode (Linear + Vibrate)")]
#[test_case("test_serveu_protocol.yaml" ; "ServeU")]
#[test_case("test_kiiroo_prowand.yaml" ; "Kiiroo ProWand Protocol")]
#[test_case("test_fleshy_thrust_protocol.yaml" ; "Fleshy Thrust Sync Protocol")]
#[tokio::test]
async fn test_device_protocols_json_v3(test_file: &str) {
  //tracing_subscriber::fmt::init();
  util::device_test::client::client_v3::run_json_test_case(&load_test_case(test_file).await).await;
}

#[test_case("test_aneros_protocol.yaml" ; "Aneros Protocol")]
#[test_case("test_ankni_protocol.yaml" ; "Ankni Protocol")]
#[test_case("test_ankni_protocol_no_handshake.yaml" ; "Ankni Protocol - No Handshake")]
#[test_case("test_cachito_protocol.yaml" ; "Cachito Protocol")]
#[test_case("test_fredorch_protocol.yaml" ; "Fredorch Protocol")]
#[test_case("test_hismith_wildolo.yaml" ; "Hismith Protocol - Wildolo")]
#[test_case("test_lovense_single_vibrator.yaml" ; "Lovense Protocol - Single Vibrator Device")]
#[test_case("test_lovense_nora.yaml" ; "Lovense Protocol - Lovense Nora (Vibrate/Rotate)")]
#[test_case("test_lovense_battery.yaml" ; "Lovense Protocol - Lovense Battery (Default Devices)")]
#[test_case("test_lovense_battery_non_default.yaml" ; "Lovense Protocol - Lovense Battery (Non-Default Devices)")]
#[test_case("test_lovense_flexer_fw2.yaml" ; "Lovense Protocol - Flexer FW2")]
#[test_case("test_lovense_edge.yaml" ; "Lovense Protocol - Edge")]
#[test_case("test_lovense_osci3.yaml" ; "Lovense Protocol - Osci3")]
#[test_case("test_satisfyer_single_vibrator.yaml" ; "Satisfyer Protocol - Single Vibrator")]
#[test_case("test_satisfyer_dual_vibrator.yaml" ; "Satisfyer Protocol - Dual Vibrator")]
#[test_case("test_satisfyer_triple_vibrator.yaml" ; "Satisfyer Protocol - Triple Vibrator")]
#[test_case("test_mysteryvibe.yaml" ; "Mysteryvibe Protocol")]
#[test_case("test_meese_protocol.yaml" ; "Meese Protocol")]
#[test_case("test_mizzzee_protocol.yaml" ; "Mizz Zee Protocol")]
#[test_case("test_mizzzee_v2_protocol.yaml" ; "Mizz Zee v2 Protocol")]
#[test_case("test_mizzzee_v3_protocol.yaml" ; "Mizz Zee v3 Protocol")]
#[test_case("test_vorze_ufo.yaml" ; "Vorze Protocol - UFO")]
#[test_case("test_vorze_ufo_tw.yaml" ; "Vorze Protocol - UFO TW")]
#[test_case("test_vorze_cyclone.yaml" ; "Vorze Protocol - Cyclone")]
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
#[test_case("test_lelo_f1sv1.yaml" ; "Lelo F1s V1 Protocol")]
#[test_case("test_lelo_f1sv2.yaml" ; "Lelo F1s V2 Protocol")]
#[test_case("test_magic_motion_1_magic_cell.yaml" ; "MagicMotion Protocol 1 - Magic Cell")]
#[test_case("test_magic_motion_2_eidolon.yaml" ; "MagicMotion Protocol 2 - Eidolon")]
#[test_case("test_magic_motion_2_equinox.yaml" ; "MagicMotion Protocol 2 - Equinox")]
#[test_case("test_magic_motion_3_krush.yaml" ; "MagicMotion Protocol 3 - Krush")]
#[test_case("test_magic_motion_4_bobi.yaml" ; "MagicMotion Protocol 4 - Bobi")]
#[test_case("test_magic_motion_4_nyx.yaml" ; "MagicMotion Protocol 4 - Nyx")]
#[test_case("test_hgod_protocol.yaml" ; "Hgod Protocol")]
#[test_case("test_metaxsire_rex.yaml" ; "metaXsire Protocol - Rex")]
#[test_case("test_sensee_protocol.yaml" ; "Sensee Diandou Protocol - Rabbit")]
#[test_case("test_svakom_pulse.yaml" ; "Svakom Pulse Protocol - Pulse Lite Neo")]
#[test_case("test_svakom_ella.yaml" ; "Svakom V1 Protocol - Ella")]
#[test_case("test_svakom_vivianna.yaml" ; "Svakom V2 Protocol - Vivianna")]
#[test_case("test_svakom_alex.yaml" ; "Svakom Alex Neo")]
#[test_case("test_svakom_alex_v2.yaml" ; "Svakom Alex Neo 2")]
#[test_case("test_svakom_iker.yaml" ; "Svakom Iker")]
#[test_case("test_fox_protocol.yaml" ; "Fox Protocol")]
#[test_case("test_xiuxiuda_protocol.yaml" ; "Xiuxiuda Protocol")]
#[test_case("test_adrienlastic_protocol.yaml" ; "Adrien Lastic Protocol")]
#[test_case("test_foreo_protocol.yaml" ; "Foreo Protocol")]
#[test_case("test_joyhub_petalwish_compat.yaml" ; "JoyHub Protocol - Petalwish Compat")]
#[test_case("test_itoys_protocol.yaml" ; "iToys Protocol")]
#[test_case("test_leten_protocol.yaml" ; "Leten Protocol")]
#[test_case("test_motorbunny_protocol.yaml" ; "Motorbunny Protocol")]
#[test_case("test_activejoy_protocol.yaml" ; "ActiveJoy Protocol")]
#[test_case("test_tryfun_surge.yaml" ; "TryFun Protocol - Surge Pro")]
#[test_case("test_cupido_protocol.yaml" ; "Cupido Protocol")]
//#[test_case("test_cowgirl_cone_protocol.yaml" ; "The Cowgirl Cone Protocol")]
#[test_case("test_amorelie_joy_protocol.yaml" ; "Amorelie Joy Protocol")]
#[test_case("test_deepsire.yaml" ; "DeepSire Protocol")]
#[test_case("test_xuanhuan_protocol.yaml" ; "Xuanhuan Protocol")]
#[test_case("test_tcode_linear_and_vibrate.yaml" ; "TCode (Linear + Vibrate)")]
#[test_case("test_serveu_protocol.yaml" ; "ServeU")]
#[test_case("test_kiiroo_prowand.yaml" ; "Kiiroo ProWand Protocol")]
#[test_case("test_fleshy_thrust_protocol.yaml" ; "Fleshy Thrust Sync Protocol")]
#[tokio::test]
async fn test_device_protocols_embedded_v2(test_file: &str) {
  util::device_test::client::client_v2::run_embedded_test_case(&load_test_case(test_file).await)
    .await;
}

#[test_case("test_aneros_protocol.yaml" ; "Aneros Protocol")]
#[test_case("test_ankni_protocol.yaml" ; "Ankni Protocol")]
#[test_case("test_ankni_protocol_no_handshake.yaml" ; "Ankni Protocol - No Handshake")]
#[test_case("test_cachito_protocol.yaml" ; "Cachito Protocol")]
#[test_case("test_fredorch_protocol.yaml" ; "Fredorch Protocol")]
#[test_case("test_hismith_wildolo.yaml" ; "Hismith Protocol - Wildolo")]
#[test_case("test_lovense_single_vibrator.yaml" ; "Lovense Protocol - Single Vibrator Device")]
#[test_case("test_lovense_nora.yaml" ; "Lovense Protocol - Lovense Nora (Vibrate/Rotate)")]
#[test_case("test_lovense_battery.yaml" ; "Lovense Protocol - Lovense Battery (Default Devices)")]
#[test_case("test_lovense_battery_non_default.yaml" ; "Lovense Protocol - Lovense Battery (Non-Default Devices)")]
#[test_case("test_lovense_edge.yaml" ; "Lovense Protocol - Edge")]
#[test_case("test_lovense_osci3.yaml" ; "Lovense Protocol - Osci3")]
#[test_case("test_satisfyer_single_vibrator.yaml" ; "Satisfyer Protocol - Single Vibrator")]
#[test_case("test_satisfyer_dual_vibrator.yaml" ; "Satisfyer Protocol - Dual Vibrator")]
#[test_case("test_satisfyer_triple_vibrator.yaml" ; "Satisfyer Protocol - Triple Vibrator")]
#[test_case("test_mysteryvibe.yaml" ; "Mysteryvibe Protocol")]
#[test_case("test_meese_protocol.yaml" ; "Meese Protocol")]
#[test_case("test_mizzzee_protocol.yaml" ; "Mizz Zee Protocol")]
#[test_case("test_mizzzee_v2_protocol.yaml" ; "Mizz Zee v2 Protocol")]
#[test_case("test_mizzzee_v3_protocol.yaml" ; "Mizz Zee v3 Protocol")]
#[test_case("test_vorze_ufo.yaml" ; "Vorze Protocol - UFO")]
#[test_case("test_vorze_ufo_tw.yaml" ; "Vorze Protocol - UFO TW")]
#[test_case("test_vorze_cyclone.yaml" ; "Vorze Protocol - Cyclone")]
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
#[test_case("test_lelo_f1sv1.yaml" ; "Lelo F1s V1 Protocol")]
#[test_case("test_lelo_f1sv2.yaml" ; "Lelo F1s V2 Protocol")]
#[test_case("test_magic_motion_1_magic_cell.yaml" ; "MagicMotion Protocol 1 - Magic Cell")]
#[test_case("test_magic_motion_2_eidolon.yaml" ; "MagicMotion Protocol 2 - Eidolon")]
#[test_case("test_magic_motion_2_equinox.yaml" ; "MagicMotion Protocol 2 - Equinox")]
#[test_case("test_magic_motion_3_krush.yaml" ; "MagicMotion Protocol 3 - Krush")]
#[test_case("test_magic_motion_4_bobi.yaml" ; "MagicMotion Protocol 4 - Bobi")]
#[test_case("test_magic_motion_4_nyx.yaml" ; "MagicMotion Protocol 4 - Nyx")]
#[test_case("test_hgod_protocol.yaml" ; "Hgod Protocol")]
#[test_case("test_metaxsire_rex.yaml" ; "metaXsire Protocol - Rex")]
#[test_case("test_sensee_protocol.yaml" ; "Sensee Diandou Protocol - Rabbit")]
#[test_case("test_svakom_pulse.yaml" ; "Svakom Pulse Protocol - Pulse Lite Neo")]
#[test_case("test_svakom_ella.yaml" ; "Svakom V1 Protocol - Ella")]
#[test_case("test_svakom_vivianna.yaml" ; "Svakom V2 Protocol - Vivianna")]
#[test_case("test_svakom_alex.yaml" ; "Svakom Alex Neo")]
#[test_case("test_svakom_alex_v2.yaml" ; "Svakom Alex Neo 2")]
#[test_case("test_svakom_iker.yaml" ; "Svakom Iker")]
#[test_case("test_fox_protocol.yaml" ; "Fox Protocol")]
#[test_case("test_xiuxiuda_protocol.yaml" ; "Xiuxiuda Protocol")]
#[test_case("test_adrienlastic_protocol.yaml" ; "Adrien Lastic Protocol")]
#[test_case("test_foreo_protocol.yaml" ; "Foreo Protocol")]
#[test_case("test_joyhub_petalwish_compat.yaml" ; "JoyHub Protocol - Petalwish Compat")]
#[test_case("test_itoys_protocol.yaml" ; "iToys Protocol")]
#[test_case("test_leten_protocol.yaml" ; "Leten Protocol")]
#[test_case("test_motorbunny_protocol.yaml" ; "Motorbunny Protocol")]
#[test_case("test_activejoy_protocol.yaml" ; "ActiveJoy Protocol")]
#[test_case("test_tryfun_surge.yaml" ; "TryFun Protocol - Surge Pro")]
#[test_case("test_cupido_protocol.yaml" ; "Cupido Protocol")]
//#[test_case("test_cowgirl_cone_protocol.yaml" ; "The Cowgirl Cone Protocol")]
#[test_case("test_amorelie_joy_protocol.yaml" ; "Amorelie Joy Protocol")]
#[test_case("test_deepsire.yaml" ; "DeepSire Protocol")]
#[test_case("test_xuanhuan_protocol.yaml" ; "Xuanhuan Protocol")]
#[test_case("test_tcode_linear_and_vibrate.yaml" ; "TCode (Linear + Vibrate)")]
#[test_case("test_serveu_protocol.yaml" ; "ServeU")]
#[test_case("test_kiiroo_prowand.yaml" ; "Kiiroo ProWand Protocol")]
#[test_case("test_fleshy_thrust_protocol.yaml" ; "Fleshy Thrust Sync Protocol")]
#[tokio::test]
async fn test_device_protocols_json_v2(test_file: &str) {
  util::device_test::client::client_v2::run_json_test_case(&load_test_case(test_file).await).await;
}
