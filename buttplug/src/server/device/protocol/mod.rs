// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Implementations of communication protocols for hardware supported by Buttplug

pub mod generic_command_manager;
// Since users can pick and choose protocols, we need all of these to be public.
pub mod aneros;
pub mod buttplug_passthru;
pub mod cachito;
pub mod hismith;
pub mod htk_bm;
pub mod lovense;

/*
pub mod ankni;
pub mod fleshlight_launch_helper;
pub mod fredorch;
pub mod hgod;

pub mod jejoue;
pub mod kiiroo_v2;
pub mod kiiroo_v21;
pub mod kiiroo_v21_initialized;
pub mod kiiroo_v2_vibrator;
pub mod lelof1s;
pub mod lelof1sv2;
pub mod libo_elle;
pub mod libo_shark;
pub mod libo_vibes;
pub mod lovedistance;
pub mod lovehoney_desire;

pub mod lovense_connect_service;
pub mod lovenuts;
pub mod magic_motion_v1;
pub mod magic_motion_v2;
pub mod magic_motion_v3;
pub mod magic_motion_v4;
pub mod mannuo;
pub mod maxpro;
pub mod mizzzee;
pub mod motorbunny;
pub mod mysteryvibe;
pub mod nobra;
pub mod patoo;
pub mod picobong;
pub mod prettylove;
pub mod raw_protocol;
pub mod realov;
pub mod satisfyer;
pub mod svakom;
pub mod svakom_alex;
pub mod svakom_iker;
pub mod svakom_sam;
pub mod tcode_v03;
pub mod thehandy;
pub mod vibratissimo;
pub mod vorze_sa;
pub mod wevibe;
pub mod wevibe8bit;
pub mod xinput;
pub mod youcups;
pub mod youou;
pub mod zalo;
*/

use crate::{
  core::{errors::ButtplugDeviceError, messages::{self, ButtplugServerMessage, ButtplugDeviceMessage, Endpoint, RawReading, ButtplugDeviceCommandMessageUnion}},
  server::{
    device::{
      configuration::{
        ProtocolAttributesType, ProtocolCommunicationSpecifier,
      },
      hardware::{Hardware, HardwareCommand, HardwareReadCmd},
      ServerDeviceIdentifier,
    },
  },
};
use futures::future::{self, BoxFuture};
use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc};

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

  add_to_protocol_map(&mut map, aneros::setup::AnerosIdentifierFactory::default());
  add_to_protocol_map(&mut map, buttplug_passthru::setup::ButtplugPassthruIdentifierFactory::default());
  add_to_protocol_map(&mut map, cachito::setup::CachitoIdentifierFactory::default());
  add_to_protocol_map(&mut map, lovense::setup::LovenseIdentifierFactory::default());
  add_to_protocol_map(&mut map, hismith::setup::HismithIdentifierFactory::default());
  add_to_protocol_map(&mut map, htk_bm::setup::HtkBmIdentifierFactory::default());
  /*
  add_to_protocol_map(&mut map, ankni::AnkniFactory::default());
  add_to_protocol_map(&mut map, fredorch::FredorchFactory::default());
  
  add_to_protocol_map(&mut map, hgod::HgodFactory::default());
  
  add_to_protocol_map(&mut map, jejoue::JeJoueFactory::default());
  add_to_protocol_map(&mut map, kiiroo_v2::KiirooV2Factory::default());
  add_to_protocol_map(&mut map, kiiroo_v2_vibrator::KiirooV2VibratorFactory::default());
  add_to_protocol_map(&mut map, kiiroo_v21::KiirooV21Factory::default());
  add_to_protocol_map(&mut map, kiiroo_v21_initialized::KiirooV21InitializedFactory::default());
  add_to_protocol_map(&mut map, lelof1s::LeloF1sFactory::default());
  add_to_protocol_map(&mut map, lelof1sv2::LeloF1sV2Factory::default());
  add_to_protocol_map(&mut map, libo_elle::LiboElleFactory::default());
  add_to_protocol_map(&mut map, libo_shark::LiboSharkFactory::default());
  add_to_protocol_map(&mut map, libo_vibes::LiboVibesFactory::default());
  add_to_protocol_map(&mut map, lovehoney_desire::LovehoneyDesireFactory::default());
  add_to_protocol_map(&mut map, lovedistance::LoveDistanceFactory::default());

  add_to_protocol_map(&mut map, lovense_connect_service::LovenseConnectServiceFactory::default());
  add_to_protocol_map(&mut map, lovenuts::LoveNutsFactory::default());
  add_to_protocol_map(&mut map, magic_motion_v1::MagicMotionV1Factory::default());
  add_to_protocol_map(&mut map, magic_motion_v2::MagicMotionV2Factory::default());
  add_to_protocol_map(&mut map, magic_motion_v3::MagicMotionV3Factory::default());
  add_to_protocol_map(&mut map, magic_motion_v4::MagicMotionV4Factory::default());
  add_to_protocol_map(&mut map, mannuo::ManNuoFactory::default());
  add_to_protocol_map(&mut map, maxpro::MaxproFactory::default());
  add_to_protocol_map(&mut map, mizzzee::MizzZeeFactory::default());
  add_to_protocol_map(&mut map, motorbunny::MotorbunnyFactory::default());
  add_to_protocol_map(&mut map, mysteryvibe::MysteryVibeFactory::default());
  add_to_protocol_map(&mut map, nobra::NobraFactory::default());
  add_to_protocol_map(&mut map, patoo::PatooFactory::default());
  add_to_protocol_map(&mut map, picobong::PicobongFactory::default());
  add_to_protocol_map(&mut map, prettylove::PrettyLoveFactory::default());
  add_to_protocol_map(&mut map, raw_protocol::RawProtocolFactory::default());
  add_to_protocol_map(&mut map, realov::RealovFactory::default());
  add_to_protocol_map(&mut map, satisfyer::SatisfyerFactory::default());
  add_to_protocol_map(&mut map, svakom::SvakomFactory::default());
  add_to_protocol_map(&mut map, svakom_alex::SvakomAlexFactory::default());
  add_to_protocol_map(&mut map, svakom_iker::SvakomIkerFactory::default());
  add_to_protocol_map(&mut map, svakom_sam::SvakomSamFactory::default());
  add_to_protocol_map(&mut map, tcode_v03::TCodeV03Factory::default());
  add_to_protocol_map(&mut map, thehandy::TheHandyFactory::default());
  add_to_protocol_map(&mut map, vibratissimo::VibratissimoFactory::default());
  add_to_protocol_map(&mut map, vorze_sa::VorzeSAFactory::default());
  add_to_protocol_map(&mut map, wevibe::WeVibeFactory::default());
  add_to_protocol_map(&mut map, wevibe8bit::WeVibe8BitFactory::default());
  add_to_protocol_map(&mut map, xinput::XInputFactory::default());
  add_to_protocol_map(&mut map, youcups::YoucupsFactory::default());
  add_to_protocol_map(&mut map, youou::YououFactory::default());
  add_to_protocol_map(&mut map, zalo::ZaloFactory::default());
  */
  map
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
  ) -> Result<(ServerDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError>;
}

#[async_trait]
pub trait ProtocolInitializer: Sync + Send {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<Box<dyn ProtocolHandler>, ButtplugDeviceError>;
}

pub struct GenericProtocolIdentifier {
  handler: Option<Box<dyn ProtocolHandler>>,
  protocol_identifier: String,
}

impl GenericProtocolIdentifier {
  pub fn new(handler: Box<dyn ProtocolHandler>, protocol_identifier: &str) -> Self {
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
  ) -> Result<(ServerDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let device_identifier = ServerDeviceIdentifier::new(
      hardware.address(),
      &self.protocol_identifier,
      &ProtocolAttributesType::Identifier(hardware.name().to_owned()),
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
  handler: Option<Box<dyn ProtocolHandler>>,
}

impl GenericProtocolInitializer {
  pub fn new(handler: Box<dyn ProtocolHandler>) -> Self {
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
  ) -> Result<Box<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(self.handler.take().unwrap())
  }
}

pub trait ProtocolHandler: Sync + Send {
  fn needs_full_command_set(&self) -> bool {
    false
  }

  fn has_handle_message(&self) -> bool {
    false
  }

  fn handle_message(
    &self,
    message: &ButtplugDeviceCommandMessageUnion,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented(print_type_of(&message))
  }

  fn command_unimplemented(
    &self,
    command: &str,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    #[cfg(build = "debug")]
    unimplemented!("Command not implemented for this protocol");
    #[cfg(not(build = "debug"))]
    Err(ButtplugDeviceError::UnhandledCommand(format!(
      "Command not implemented for this protocol: {}",
      command
    )))
  }

  fn handle_level_cmd(
    &self,
    message: messages::LevelCmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_vorze_a10_cyclone_cmd(
    &self,
    message: messages::VorzeA10CycloneCmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_kiiroo_cmd(
    &self,
    message: messages::KiirooCmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_fleshlight_launch_fw12_cmd(
    &self,
    message: messages::FleshlightLaunchFW12Cmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_vibrate_cmd(
    &self,
    message: &Vec<Option<u32>>,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_rotate_cmd(
    &self,
    message: &Vec<Option<(u32, bool)>>,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_linear_cmd(
    &self,
    message: messages::LinearCmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::BatteryLevelCmd,
  ) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
    // If we have a standardized BLE Battery endpoint, handle that above the
    // protocol, as it'll always be the same.
    if device.endpoints().contains(&Endpoint::RxBLEBattery) {
      info!("Trying to get battery reading.");
      let msg = HardwareReadCmd::new(Endpoint::RxBLEBattery, 1, 0);
      let fut = device.read_value(&msg);
      Box::pin(async move {
        let raw_msg: RawReading = fut.await?;
        let battery_level = raw_msg.data()[0] as f64 / 100f64;
        let battery_reading =
          messages::BatteryLevelReading::new(message.device_index(), battery_level);
        info!("Got battery reading: {}", battery_level);
        Ok(battery_reading.into())
      })
    } else {
      Box::pin(future::ready(Err(ButtplugDeviceError::UnhandledCommand(format!("Command not implemented for this protocol: BatteryCmd")))))
    }
  }

  fn handle_rssi_level_cmd(
    &self,
    message: messages::RSSILevelCmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented(print_type_of(&message))
  }
}

#[macro_export]
macro_rules! generic_protocol_setup {
  ( $protocol_name:ident, $protocol_identifier:tt) => {
    paste::paste! {
      pub mod setup {
        use crate::server::device::protocol::{
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
              Box::new(super::$protocol_name::default()),
              self.identifier(),
            ))
          }
        }
      }
    }
  }
}

pub use generic_protocol_setup;