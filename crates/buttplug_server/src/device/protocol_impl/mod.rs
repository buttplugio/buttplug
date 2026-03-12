// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::protocol::ProtocolIdentifier;

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
pub mod fluffer;
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
pub mod kiiroo_powershot;
pub mod kiiroo_prowand;
pub mod kiiroo_spot;
pub mod kiiroo_v2;
pub mod kiiroo_v21;
pub mod kiiroo_v21_initialized;
pub mod kiiroo_v2_vibrator;
pub mod kiiroo_v3;
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
pub mod lovense_connect_service;
pub mod lovenuts;
pub mod luvmazer;
pub mod magic_motion_v1;
pub mod magic_motion_v2;
pub mod magic_motion_v3;
pub mod magic_motion_v4;
pub mod mannuo;
pub mod maxpro;
pub mod meese;
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
pub mod sexverse_v1;
pub mod sexverse_v2;
pub mod sexverse_v3;
pub mod sexverse_v4;
pub mod sexverse_v5;
pub mod svakom;
pub mod synchro;
pub mod tcode_v03;
pub mod thehandy;
pub mod thehandy_v3;
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

macro_rules! protocol_match {
  ( $var:ident, $( $( $protocol_module:ident )::+ ),* $(,)? ) => {
    match $var {
      $(
        $( $protocol_module )::+::setup::IDENTIFIER => Some($( $protocol_module )::+::setup::create_identifier()),
      )*
      _ => None,
    }
  };
}

pub fn get_protocol_identifier(protocol_identifier: &str) -> Option<Box<dyn ProtocolIdentifier>> {
  protocol_match!(
    protocol_identifier,
    activejoy,
    adrienlastic,
    amorelie_joy,
    aneros,
    ankni,
    bananasome,
    cachito,
    cowgirl,
    cowgirl_cone,
    cupido,
    deepsire,
    feelingso,
    fleshy_thrust,
    fluffer,
    foreo,
    fox,
    fredorch,
    fredorch_rotary,
    galaku,
    galaku_pump,
    hgod,
    hismith,
    hismith_mini,
    htk_bm,
    itoys,
    jejoue,
    joyhub,
    kgoal_boost,
    kiiroo_powershot,
    kiiroo_prowand,
    kiiroo_spot,
    kiiroo_v2,
    kiiroo_v21,
    kiiroo_v21_initialized,
    kiiroo_v2_vibrator,
    kiiroo_v3,
    kizuna,
    lelo_harmony,
    lelof1s,
    lelof1sv2,
    leten,
    libo_elle,
    libo_shark,
    libo_vibes,
    lioness,
    loob,
    lovedistance,
    lovehoney_desire,
    lovense,
    lovense_connect_service,
    lovenuts,
    luvmazer,
    magic_motion_v1,
    magic_motion_v2,
    magic_motion_v3,
    magic_motion_v4,
    mannuo,
    maxpro,
    meese,
    mizzzee,
    mizzzee_v2,
    mizzzee_v3,
    monsterpub,
    motorbunny,
    mysteryvibe,
    mysteryvibe_v2,
    nextlevelracing,
    nexus_revo,
    nintendo_joycon,
    nobra,
    omobo,
    patoo,
    picobong,
    pink_punch,
    prettylove,
    raw_protocol,
    realov,
    sakuraneko,
    satisfyer,
    sensee,
    sensee_capsule,
    sensee_v2,
    serveu,
    sexverse_lg389,
    sexverse_v1,
    sexverse_v2,
    sexverse_v3,
    sexverse_v4,
    sexverse_v5,
    svakom::svakom_alex,
    svakom::svakom_alex_v2,
    svakom::svakom_avaneo,
    svakom::svakom_barnard,
    svakom::svakom_barney,
    svakom::svakom_dice,
    svakom::svakom_dt250a,
    svakom::svakom_iker,
    svakom::svakom_jordan,
    svakom::svakom_pulse,
    svakom::svakom_sam,
    svakom::svakom_sam2,
    svakom::svakom_suitcase,
    svakom::svakom_tarax,
    svakom::svakom_v1,
    svakom::svakom_v2,
    svakom::svakom_v3,
    svakom::svakom_v4,
    svakom::svakom_v5,
    svakom::svakom_v6,
    synchro,
    tcode_v03,
    thehandy,
    thehandy_v3,
    tryfun,
    tryfun_blackhole,
    tryfun_meta2,
    vibcrafter,
    vibratissimo,
    vorze_sa,
    wetoy,
    wevibe,
    wevibe8bit,
    wevibe_chorus,
    xibao,
    xinput,
    xiuxiuda,
    xuanhuan,
    youcups,
    youou,
    zalo
  )
}
