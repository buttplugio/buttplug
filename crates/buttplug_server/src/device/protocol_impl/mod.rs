use std::{collections::HashMap, sync::Arc};

use crate::device::protocol::ProtocolIdentifierFactory;


// Utility mods
pub mod fleshlight_launch_helper;

// Since users can pick and choose protocols, we need all of these to be public.
pub mod activejoy;
pub mod adrienlastic;
pub mod amorelie_joy;
pub mod aneros;
pub mod ankni;
pub mod bananasome;
pub mod cachito;
pub mod cowgirl;
pub mod cowgirl_cone;
pub mod cupido;
pub mod deepsire;
pub mod feelingso;
pub mod fleshy_thrust;
pub mod foreo;
pub mod fox;
pub mod fredorch;
pub mod fredorch_rotary;
pub mod galaku;
pub mod galaku_pump;
pub mod hgod;
pub mod hismith;
pub mod hismith_mini;
pub mod htk_bm;
pub mod itoys;
pub mod jejoue;
pub mod joyhub;
pub mod kgoal_boost;
pub mod kiiroo_prowand;
pub mod kiiroo_spot;
pub mod kiiroo_v2;
pub mod kiiroo_v21;
pub mod kiiroo_v21_initialized;
pub mod kiiroo_v2_vibrator;
pub mod kizuna;
pub mod lelo_harmony;
pub mod lelof1s;
pub mod lelof1sv2;
pub mod leten;
pub mod libo_elle;
pub mod libo_shark;
pub mod libo_vibes;
pub mod lioness;
pub mod loob;
pub mod lovedistance;
pub mod lovehoney_desire;
pub mod lovense;
// pub mod lovense_connect_service;
pub mod lovenuts;
pub mod luvmazer;
pub mod magic_motion_v1;
pub mod magic_motion_v2;
pub mod magic_motion_v3;
pub mod magic_motion_v4;
pub mod mannuo;
pub mod maxpro;
pub mod meese;
pub mod metaxsire;
pub mod metaxsire_v2;
pub mod metaxsire_v3;
pub mod metaxsire_v4;
pub mod metaxsire_v5;
pub mod mizzzee;
pub mod mizzzee_v2;
pub mod mizzzee_v3;
pub mod monsterpub;
pub mod motorbunny;
pub mod mysteryvibe;
pub mod mysteryvibe_v2;
pub mod nextlevelracing;
pub mod nexus_revo;
pub mod nintendo_joycon;
pub mod nobra;
pub mod omobo;
pub mod patoo;
pub mod picobong;
pub mod pink_punch;
pub mod prettylove;
pub mod raw_protocol;
pub mod realov;
pub mod sakuraneko;
pub mod satisfyer;
pub mod sensee;
pub mod sensee_capsule;
pub mod sensee_v2;
pub mod serveu;
pub mod sexverse_lg389;
pub mod svakom;
pub mod synchro;
pub mod tcode_v03;
pub mod thehandy;
pub mod tryfun;
pub mod tryfun_blackhole;
pub mod tryfun_meta2;
pub mod vibcrafter;
pub mod vibratissimo;
pub mod vorze_sa;
pub mod wetoy;
pub mod wevibe;
pub mod wevibe8bit;
pub mod wevibe_chorus;
pub mod xibao;
pub mod xinput;
pub mod xiuxiuda;
pub mod xuanhuan;
pub mod youcups;
pub mod youou;
pub mod zalo;

pub fn get_default_protocol_map() -> HashMap<String, Arc<dyn ProtocolIdentifierFactory>> {
  let mut map = HashMap::new();
  fn add_to_protocol_map<T>(
    map: &mut HashMap<String, Arc<dyn ProtocolIdentifierFactory>>,
    factory: T,
  ) where
    T: ProtocolIdentifierFactory + 'static,
  {
    let factory = Arc::new(factory);
    map.insert(factory.identifier().to_owned(), factory);
  }

  add_to_protocol_map(
    &mut map,
    activejoy::setup::ActiveJoyIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    adrienlastic::setup::AdrienLasticIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    amorelie_joy::setup::AmorelieJoyIdentifierFactory::default(),
  );
  add_to_protocol_map(&mut map, aneros::setup::AnerosIdentifierFactory::default());
  add_to_protocol_map(&mut map, ankni::setup::AnkniIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    bananasome::setup::BananasomeIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    cachito::setup::CachitoIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    cowgirl::setup::CowgirlIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    cowgirl_cone::setup::CowgirlConeIdentifierFactory::default(),
  );
  add_to_protocol_map(&mut map, cupido::setup::CupidoIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    deepsire::setup::DeepSireIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    lovense::setup::LovenseIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    hismith::setup::HismithIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    hismith_mini::setup::HismithMiniIdentifierFactory::default(),
  );
  add_to_protocol_map(&mut map, htk_bm::setup::HtkBmIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    thehandy::setup::TheHandyIdentifierFactory::default(),
  );

  add_to_protocol_map(
    &mut map,
    feelingso::setup::FeelingSoIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    fleshy_thrust::setup::FleshyThrustIdentifierFactory::default(),
  );
  add_to_protocol_map(&mut map, foreo::setup::ForeoIdentifierFactory::default());
  add_to_protocol_map(&mut map, fox::setup::FoxIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    fredorch::setup::FredorchIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    fredorch_rotary::setup::FredorchRotaryIdentifierFactory::default(),
  );

  add_to_protocol_map(&mut map, hgod::setup::HgodIdentifierFactory::default());

  add_to_protocol_map(
    &mut map,
    galaku_pump::setup::GalakuPumpIdentifierFactory::default(),
  );

  add_to_protocol_map(&mut map, galaku::setup::GalakuIdentifierFactory::default());

  add_to_protocol_map(&mut map, itoys::setup::IToysIdentifierFactory::default());
  add_to_protocol_map(&mut map, jejoue::setup::JeJoueIdentifierFactory::default());
  add_to_protocol_map(&mut map, joyhub::joyhub::setup::JoyHubIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    joyhub::joyhub_v2::setup::JoyHubV2IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    joyhub::joyhub_v3::setup::JoyHubV3IdentifierFactory::default(),
  );

  add_to_protocol_map(
    &mut map,
    joyhub::joyhub_v4::setup::JoyHubV4IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    joyhub::joyhub_v5::setup::JoyHubV5IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    joyhub::joyhub_v6::setup::JoyHubV6IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    kiiroo_prowand::setup::KiirooProWandIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    kiiroo_spot::setup::KiirooSpotIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    kiiroo_v2::setup::KiirooV2IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    kiiroo_v2_vibrator::setup::KiirooV2VibratorIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    kiiroo_v21::setup::KiirooV21IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    kiiroo_v21_initialized::setup::KiirooV21InitializedIdentifierFactory::default(),
  );
  add_to_protocol_map(&mut map, kizuna::setup::KizunaIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    lelof1s::setup::LeloF1sIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    lelof1sv2::setup::LeloF1sV2IdentifierFactory::default(),
  );
  add_to_protocol_map(&mut map, leten::setup::LetenIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    lelo_harmony::setup::LeloHarmonyIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    libo_elle::setup::LiboElleIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    libo_shark::setup::LiboSharkIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    libo_vibes::setup::LiboVibesIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    lioness::setup::LionessIdentifierFactory::default(),
  );
  add_to_protocol_map(&mut map, loob::setup::LoobIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    lovehoney_desire::setup::LovehoneyDesireIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    lovedistance::setup::LoveDistanceIdentifierFactory::default(),
  );
  //  add_to_protocol_map(
  //    &mut map,
  //    lovense_connect_service::setup::LovenseConnectServiceIdentifierFactory::default(),
  //  );
  add_to_protocol_map(
    &mut map,
    lovenuts::setup::LoveNutsIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    luvmazer::setup::LuvmazerIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    magic_motion_v1::setup::MagicMotionV1IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    magic_motion_v2::setup::MagicMotionV2IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    magic_motion_v3::setup::MagicMotionV3IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    magic_motion_v4::setup::MagicMotionV4IdentifierFactory::default(),
  );
  add_to_protocol_map(&mut map, mannuo::setup::ManNuoIdentifierFactory::default());
  add_to_protocol_map(&mut map, maxpro::setup::MaxproIdentifierFactory::default());
  add_to_protocol_map(&mut map, meese::setup::MeeseIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    metaxsire::setup::MetaXSireIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    metaxsire_v2::setup::MetaXSireV2IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    metaxsire_v3::setup::MetaXSireV3IdentifierFactory::default(),
  );
    add_to_protocol_map(
        &mut map,
        metaxsire_v4::setup::MetaXSireV4IdentifierFactory::default(),
    );
    add_to_protocol_map(
        &mut map,
        metaxsire_v5::setup::MetaXSireV5IdentifierFactory::default(),
    );
  add_to_protocol_map(
    &mut map,
    mizzzee::setup::MizzZeeIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    mizzzee_v2::setup::MizzZeeV2IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    mizzzee_v3::setup::MizzZeeV3IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    monsterpub::setup::MonsterPubIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    motorbunny::setup::MotorbunnyIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    mysteryvibe::setup::MysteryVibeIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    mysteryvibe_v2::setup::MysteryVibeV2IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    nexus_revo::setup::NexusRevoIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    nextlevelracing::setup::NextLevelRacingIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    nintendo_joycon::setup::NintendoJoyconIdentifierFactory::default(),
  );
  add_to_protocol_map(&mut map, nobra::setup::NobraIdentifierFactory::default());
  add_to_protocol_map(&mut map, omobo::setup::OmoboIdentifierFactory::default());
  add_to_protocol_map(&mut map, patoo::setup::PatooIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    picobong::setup::PicobongIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    pink_punch::setup::PinkPunchIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    prettylove::setup::PrettyLoveIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    raw_protocol::setup::RawProtocolIdentifierFactory::default(),
  );
  add_to_protocol_map(&mut map, realov::setup::RealovIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    sakuraneko::setup::SakuranekoIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    satisfyer::setup::SatisfyerIdentifierFactory::default(),
  );
  add_to_protocol_map(&mut map, sensee::setup::SenseeIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    sensee_capsule::setup::SenseeCapsuleIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    sensee_v2::setup::SenseeV2IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    sexverse_lg389::setup::SexverseLG389IdentifierFactory::default(),
  );
  add_to_protocol_map(&mut map, serveu::setup::ServeUIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    svakom::svakom_avaneo::setup::SvakomAvaNeoIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_alex::setup::SvakomAlexIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_alex_v2::setup::SvakomAlexV2IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_barnard::setup::SvakomBarnardIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_barney::setup::SvakomBarneyIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_dice::setup::SvakomDiceIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_dt250a::setup::SvakomDT250AIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_iker::setup::SvakomIkerIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_jordan::setup::SvakomJordanIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_pulse::setup::SvakomPulseIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_sam::setup::SvakomSamIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_sam2::setup::SvakomSam2IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_suitcase::setup::SvakomSuitcaseIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_tarax::setup::SvakomTaraXIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_v1::setup::SvakomV1IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_v2::setup::SvakomV2IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_v3::setup::SvakomV3IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_v4::setup::SvakomV4IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_v5::setup::SvakomV5IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    svakom::svakom_v6::setup::SvakomV6IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    synchro::setup::SynchroIdentifierFactory::default(),
  );
  add_to_protocol_map(&mut map, tryfun::setup::TryFunIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    tryfun_blackhole::setup::TryFunBlackHoleIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    tryfun_meta2::setup::TryFunMeta2IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    tcode_v03::setup::TCodeV03IdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    vibcrafter::setup::VibCrafterIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    vibratissimo::setup::VibratissimoIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    vorze_sa::setup::VorzeSAIdentifierFactory::default(),
  );
  add_to_protocol_map(&mut map, wetoy::setup::WeToyIdentifierFactory::default());
  add_to_protocol_map(&mut map, wevibe::setup::WeVibeIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    wevibe8bit::setup::WeVibe8BitIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    wevibe_chorus::setup::WeVibeChorusIdentifierFactory::default(),
  );
  add_to_protocol_map(&mut map, xibao::setup::XibaoIdentifierFactory::default());
  add_to_protocol_map(&mut map, xinput::setup::XInputIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    xiuxiuda::setup::XiuxiudaIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    xuanhuan::setup::XuanhuanIdentifierFactory::default(),
  );
  add_to_protocol_map(
    &mut map,
    youcups::setup::YoucupsIdentifierFactory::default(),
  );
  add_to_protocol_map(&mut map, youou::setup::YououIdentifierFactory::default());
  add_to_protocol_map(&mut map, zalo::setup::ZaloIdentifierFactory::default());
  add_to_protocol_map(
    &mut map,
    kgoal_boost::setup::KGoalBoostIdentifierFactory::default(),
  );
  map
}
