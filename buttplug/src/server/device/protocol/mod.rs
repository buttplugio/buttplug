// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Implementations of communication protocols for hardware supported by Buttplug

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
// pub mod joyhub;
// pub mod joyhub_v2;
pub mod joyhub_v3;
// pub mod joyhub_v4;
// pub mod joyhub_v5;
// pub mod joyhub_v6;
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
mod metaxsire_v4;
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

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{OutputCommand, Endpoint, InputReadingV4, InputType},
  },
  server::{
    device::{
      configuration::{ProtocolCommunicationSpecifier, UserDeviceDefinition, UserDeviceIdentifier},
      hardware::{Hardware, HardwareCommand, HardwareReadCmd},
    },
    message::{
      checked_output_cmd::CheckedOutputCmdV4,
      spec_enums::ButtplugDeviceCommandMessageUnionV4,
      ButtplugServerDeviceMessage,
    },
  },
};
use async_trait::async_trait;
use futures::{
  future::{self, BoxFuture, FutureExt},
  StreamExt,
};
use std::{pin::Pin, time::Duration};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

/// Strategy for situations where hardware needs to get updates every so often in order to keep
/// things alive. Currently this applies to iOS backgrounding with bluetooth devices, as well as
/// some protocols like Satisfyer and Mysteryvibe that need constant command refreshing, but since
/// we never know which of our hundreds of supported devices someone might connect, we need context
/// as to which keepalive strategy to use.
///
/// When choosing a keepalive strategy for a protocol:
///
/// - If the protocol has a command that essentially does nothing to the actuators, set up
///   RepeatPacketStrategy to use that. This is useful for devices that have info commands (like
///   Lovense), ping commands (like The Handy), sensor commands that aren't yet subscribed to output
///   notifications, etc...
/// - If a protocol needs specific timing or keepalives, regardless of the OS/hardware manager being
///   used, like Satisfyer or Mysteryvibe, use RepeatLastPacketStrategyWithTiming.
/// - For many devices with only scalar actuators, RepeatLastPacketStrategy should work. You just
///   need to make sure the protocol doesn't have a packet counter or something else that will trip
///   if the same packet is replayed multiple times.
#[derive(Debug)]
pub enum ProtocolKeepaliveStrategy {
  /// Repeat a specific packet, such as a ping or a no-op. Only do this when the hardware manager
  /// requires it (currently only iOS bluetooth during backgrounding).
  HardwareRequiredRepeatPacketStrategy(HardwareWriteCmd),
  /// Repeat whatever the last packet sent was, and send Stop commands until first packet sent. Uses
  /// a default timing, suitable for most protocols that don't need constant device updates outside
  /// of OS requirements. Only do this when the hardware manager requires it (currently only iOS
  /// bluetooth during backgrounding).
  HardwareRequiredRepeatLastPacketStrategy,
  /// Repeat whatever the last packet sent was, and send Stop commands until first packet sent. Do
  /// this regardless of whether or not the hardware manager requires it. Useful for hardware that
  /// requires keepalives, like Satisfyer, Mysteryvibe, Leten, etc...
  RepeatLastPacketStrategyWithTiming(Duration),
}

pub trait ProtocolIdentifierFactory: Send + Sync {
  fn identifier(&self) -> &str;
  fn create(&self) -> Box<dyn ProtocolIdentifier>;
}

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
  //  add_to_protocol_map(&mut map, joyhub::setup::JoyHubIdentifierFactory::default());
  //  add_to_protocol_map(
  //    &mut map,
  //    joyhub_v2::setup::JoyHubV2IdentifierFactory::default(),
  //  );

  add_to_protocol_map(
    &mut map,
    joyhub_v3::setup::JoyHubV3IdentifierFactory::default(),
  );

  //  add_to_protocol_map(
  //    &mut map,
  //    joyhub_v4::setup::JoyHubV4IdentifierFactory::default(),
  //  );
  //  add_to_protocol_map(
  //    &mut map,
  //    joyhub_v5::setup::JoyHubV5IdentifierFactory::default(),
  //  );
  //  add_to_protocol_map(
  //    &mut map,
  //    joyhub_v6::setup::JoyHubV6IdentifierFactory::default(),
  //  );
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
  //add_to_protocol_map(
  //  &mut map,
  //  svakom::svakom_avaneo::setup::SvakomAvaNeoIdentifierFactory::default(),
  //);
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
  //add_to_protocol_map(
  //  &mut map,
  //  svakom::svakom_dt250a::setup::SvakomDT250AIdentifierFactory::default(),
  //);
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
  //add_to_protocol_map(
  //  &mut map,
  //  svakom::svakom_suitcase::setup::SvakomSuitcaseIdentifierFactory::default(),
  //);
  //add_to_protocol_map(
  //  &mut map,
  //  svakom::svakom_tarax::setup::SvakomTaraXIdentifierFactory::default(),
  //);
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

pub enum ProtocolValueCommandPrefilterStrategy {
  /// Drop repeated ValueCmd/ValueWithParameterCmd messages
  DropRepeats,
  /// No filter, send all value messages for processing
  None,
}

fn print_type_of<T>(_: &T) -> &'static str {
  std::any::type_name::<T>()
}

pub struct ProtocolSpecializer {
  specifiers: Vec<ProtocolCommunicationSpecifier>,
  identifier: Box<dyn ProtocolIdentifier>,
}

impl ProtocolSpecializer {
  pub fn new(
    specifiers: Vec<ProtocolCommunicationSpecifier>,
    identifier: Box<dyn ProtocolIdentifier>,
  ) -> Self {
    Self {
      specifiers,
      identifier,
    }
  }

  pub fn specifiers(&self) -> &Vec<ProtocolCommunicationSpecifier> {
    &self.specifiers
  }

  pub fn identify(self) -> Box<dyn ProtocolIdentifier> {
    self.identifier
  }
}

#[async_trait]
pub trait ProtocolIdentifier: Sync + Send {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    specifier: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError>;
}

#[async_trait]
pub trait ProtocolInitializer: Sync + Send {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    device_definition: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError>;
}

pub struct GenericProtocolIdentifier {
  handler: Option<Arc<dyn ProtocolHandler>>,
  protocol_identifier: String,
}

impl GenericProtocolIdentifier {
  pub fn new(handler: Arc<dyn ProtocolHandler>, protocol_identifier: &str) -> Self {
    Self {
      handler: Some(handler),
      protocol_identifier: protocol_identifier.to_owned(),
    }
  }
}

#[async_trait]
impl ProtocolIdentifier for GenericProtocolIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let device_identifier = UserDeviceIdentifier::new(
      hardware.address(),
      &self.protocol_identifier,
      &Some(hardware.name().to_owned()),
    );
    Ok((
      device_identifier,
      Box::new(GenericProtocolInitializer::new(
        self.handler.take().unwrap(),
      )),
    ))
  }
}

pub struct GenericProtocolInitializer {
  handler: Option<Arc<dyn ProtocolHandler>>,
}

impl GenericProtocolInitializer {
  pub fn new(handler: Arc<dyn ProtocolHandler>) -> Self {
    Self {
      handler: Some(handler),
    }
  }
}

#[async_trait]
impl ProtocolInitializer for GenericProtocolInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(self.handler.take().unwrap())
  }
}

pub trait ProtocolHandler: Sync + Send {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
  }

  fn handle_message(
    &self,
    message: &ButtplugDeviceCommandMessageUnionV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented(print_type_of(&message))
  }

  // Allow here since this changes between debug/release
  #[allow(unused_variables)]
  fn command_unimplemented(
    &self,
    command: &str,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    #[cfg(debug_assertions)]
    unimplemented!("Command not implemented for this protocol");
    #[cfg(not(debug_assertions))]
    Err(ButtplugDeviceError::UnhandledCommand(format!(
      "Command not implemented for this protocol: {}",
      command
    )))
  }

  // The default scalar handler assumes that most devices require discrete commands per feature. If
  // a protocol has commands that combine multiple features, either with matched or unmatched
  // actuators, they should just implement their own version of this method.
  fn handle_output_cmd(
    &self,
    cmd: &CheckedOutputCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let output_command = cmd.output_command();
    match output_command {
      OutputCommand::Constrict(x) => {
        self.handle_output_constrict_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
      }
      OutputCommand::Inflate(x) => {
        self.handle_output_inflate_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
      }
      OutputCommand::Oscillate(x) => {
        self.handle_output_oscillate_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
      }
      OutputCommand::Rotate(x) => {
        self.handle_output_rotate_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
      }
      OutputCommand::Vibrate(x) => {
        self.handle_output_vibrate_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
      }
      OutputCommand::Position(x) => {
        self.handle_output_position_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
      }
      OutputCommand::Heater(x) => {
        self.handle_output_heater_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
      }
      OutputCommand::Led(x) => {
        self.handle_output_led_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
      }
      OutputCommand::PositionWithDuration(x) => self.handle_position_with_duration_cmd(
        cmd.feature_index(),
        cmd.feature_id(),
        x.position(),
        x.duration(),
      ),
      OutputCommand::RotateWithDirection(x) => self.handle_rotation_with_direction_cmd(
        cmd.feature_index(),
        cmd.feature_id(),
        x.speed(),
        x.clockwise(),
      ),
    }
  }

  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Vibrate Actuator)")
  }

  fn handle_output_rotate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Rotate Actuator)")
  }

  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Oscillate Actuator)")
  }

  fn handle_output_inflate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Inflate Actuator)")
  }

  fn handle_output_constrict_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Constrict Actuator)")
  }

  fn handle_output_heater_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Heater Actuator)")
  }

  fn handle_output_led_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Led Actuator)")
  }

  fn handle_output_position_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _position: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Position Actuator)")
  }

  fn handle_position_with_duration_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _position: u32,
    _duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Position w/ Duration Actuator)")
  }

  fn handle_rotation_with_direction_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _speed: u32,
    _clockwise: bool,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Rotation w/ Direction Actuator)")
  }

  fn handle_input_subscribe_cmd(
    &self,
    _device: Arc<Hardware>,
    _feature_index: u32,
    _feature_id: Uuid,
    _sensor_type: InputType,
  ) -> BoxFuture<Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "Command not implemented for this protocol: InputCmd (Subscribe)".to_string(),
    )))
    .boxed()
  }

  fn handle_input_unsubscribe_cmd(
    &self,
    _device: Arc<Hardware>,
    _feature_index: u32,
    _feature_id: Uuid,
    _sensor_type: InputType,
  ) -> BoxFuture<Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "Command not implemented for this protocol: InputCmd (Unsubscribe)".to_string(),
    )))
    .boxed()
  }

  fn handle_input_read_cmd(
    &self,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
    sensor_type: InputType,
  ) -> BoxFuture<Result<InputReadingV4, ButtplugDeviceError>> {
    match sensor_type {
      InputType::Battery => self.handle_battery_level_cmd(device, feature_index, feature_id),
      _ => future::ready(Err(ButtplugDeviceError::UnhandledCommand(
        "Command not implemented for this protocol: InputCmd (Read)".to_string(),
      )))
      .boxed(),
    }
  }

  // Handle Battery Level returns a SensorReading, as we'll always need to do a sensor index
  // conversion on it.
  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
  ) -> BoxFuture<Result<InputReadingV4, ButtplugDeviceError>> {
    // If we have a standardized BLE Battery endpoint, handle that above the
    // protocol, as it'll always be the same.
    if device.endpoints().contains(&Endpoint::RxBLEBattery) {
      debug!("Trying to get battery reading.");
      let msg = HardwareReadCmd::new(feature_id, Endpoint::RxBLEBattery, 1, 0);
      let fut = device.read_value(&msg);
      async move {
        let hw_msg = fut.await?;
        let battery_level = hw_msg.data()[0] as i32;
        let battery_reading =
          InputReadingV4::new(0, feature_index, InputType::Battery, vec![battery_level]);
        debug!("Got battery reading: {}", battery_level);
        Ok(battery_reading)
      }
      .boxed()
    } else {
      future::ready(Err(ButtplugDeviceError::UnhandledCommand(
        "Command not implemented for this protocol: SensorReadCmd".to_string(),
      )))
      .boxed()
    }
  }

  fn handle_rssi_level_cmd(
    &self,
    _device: Arc<Hardware>,
    _feature_index: u32,
    _feature_id: Uuid,
  ) -> BoxFuture<Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "Command not implemented for this protocol: SensorReadCmd".to_string(),
    )))
    .boxed()
  }

  fn event_stream(
    &self,
  ) -> Pin<Box<dyn tokio_stream::Stream<Item = ButtplugServerDeviceMessage> + Send>> {
    tokio_stream::empty().boxed()
  }
}

#[macro_export]
macro_rules! generic_protocol_setup {
  ( $protocol_name:ident, $protocol_identifier:tt) => {
    paste::paste! {
      pub mod setup {
        use std::sync::Arc;
        use $crate::server::device::protocol::{
          GenericProtocolIdentifier, ProtocolIdentifier, ProtocolIdentifierFactory,
        };
        #[derive(Default)]
        pub struct [< $protocol_name IdentifierFactory >] {}

        impl ProtocolIdentifierFactory for  [< $protocol_name IdentifierFactory >] {
          fn identifier(&self) -> &str {
            $protocol_identifier
          }

          fn create(&self) -> Box<dyn ProtocolIdentifier> {
            Box::new(GenericProtocolIdentifier::new(
              Arc::new(super::$protocol_name::default()),
              self.identifier(),
            ))
          }
        }
      }
    }
  };
}

#[macro_export]
macro_rules! generic_protocol_initializer_setup {
  ( $protocol_name:ident, $protocol_identifier:tt) => {
    paste::paste! {
      pub mod setup {
        use $crate::server::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
        #[derive(Default)]
        pub struct [< $protocol_name IdentifierFactory >] {}

        impl ProtocolIdentifierFactory for [< $protocol_name IdentifierFactory >] {
          fn identifier(&self) -> &str {
            $protocol_identifier
          }

          fn create(&self) -> Box<dyn ProtocolIdentifier> {
            Box::new(super::[< $protocol_name Identifier >]::default())
          }
        }
      }

      #[derive(Default)]
      pub struct [< $protocol_name Identifier >] {}

      #[async_trait]
      impl ProtocolIdentifier for [< $protocol_name Identifier >] {
        async fn identify(
          &mut self,
          hardware: Arc<Hardware>,
          _: ProtocolCommunicationSpecifier,
        ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
          Ok((UserDeviceIdentifier::new(hardware.address(), $protocol_identifier, &Some(hardware.name().to_owned())), Box::new([< $protocol_name Initializer >]::default())))
        }
      }
    }
  };
}

pub use generic_protocol_initializer_setup;
pub use generic_protocol_setup;

use super::hardware::HardwareWriteCmd;
