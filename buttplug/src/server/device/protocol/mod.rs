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
pub mod ankni;
pub mod buttplug_passthru;
pub mod cachito;
pub mod fleshlight_launch_helper;
pub mod fredorch;

pub mod hgod;
pub mod hismith;
pub mod htk_bm;
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
pub mod lovense;
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

use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::{
      self, ButtplugDeviceCommandMessageUnion, ButtplugDeviceMessage, ButtplugDeviceMessageType,
      ButtplugMessage, Endpoint, RawReading, VibrateCmd, VibrateSubcommand,
    },
  },
  server::{
    ButtplugServerResultFuture,
    device::{
      configuration::{ProtocolDeviceAttributesBuilder, ProtocolDeviceAttributes, ProtocolAttributesIdentifier},
      hardware::{HardwareReadCmd, Hardware},
    },
  },
};
use futures::future::{self, BoxFuture};
use generic_command_manager::GenericCommandManager;
use std::{
  collections::HashMap,
  sync::Arc,
};

pub fn get_default_protocol_map() -> HashMap<String, Arc<dyn ButtplugProtocolFactory>> {
  let mut map = HashMap::new();
  fn add_to_protocol_map<T>(map: &mut HashMap<String, Arc<dyn ButtplugProtocolFactory>>, factory: T)
    where
  T: ButtplugProtocolFactory + 'static
  {
    let factory = Arc::new(factory);
    map.insert(
      factory.protocol_identifier().to_owned(),
      factory
    );
  }

  add_to_protocol_map(&mut map, aneros::AnerosFactory::default());
  add_to_protocol_map(&mut map, ankni::AnkniFactory::default());
  add_to_protocol_map(&mut map, buttplug_passthru::ButtplugPassthruFactory::default());
  add_to_protocol_map(&mut map, cachito::CachitoFactory::default());
  add_to_protocol_map(&mut map, fredorch::FredorchFactory::default());
  add_to_protocol_map(&mut map, hismith::HismithFactory::default());
  add_to_protocol_map(&mut map, hgod::HgodFactory::default());
  add_to_protocol_map(&mut map, htk_bm::HtkBmFactory::default());
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
  add_to_protocol_map(&mut map, lovense::LovenseFactory::default());
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
  map
}

pub trait ButtplugProtocolFactory: std::fmt::Debug + Send + Sync {
  fn protocol_identifier(&self) -> &'static str;

  fn try_create(
    &self,
    hardware: Arc<Hardware>,
    attributes_builder: ProtocolDeviceAttributesBuilder,
  ) -> BoxFuture<'static, Result<Box<dyn ButtplugProtocol>, ButtplugError>>;
}

pub trait ButtplugProtocolProperties {
  fn name(&self) -> &str;
  fn protocol_identifier(&self) -> &str;
  fn protocol_attributes_identifier(&self) -> &ProtocolAttributesIdentifier { 
    self.device_attributes().identifier()
  }
  fn device_attributes(&self) -> &ProtocolDeviceAttributes;
  fn stop_commands(&self) -> Vec<ButtplugDeviceCommandMessageUnion>;

  fn supports_message(
    &self,
    message: &ButtplugDeviceCommandMessageUnion,
  ) -> Result<(), ButtplugError> {
    // TODO This should be generated by a macro, as should the types enum.
    match message {
      ButtplugDeviceCommandMessageUnion::BatteryLevelCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::BatteryLevelCmd)
        .then(|| ())
        .ok_or(ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::BatteryLevelCmd)),
      ButtplugDeviceCommandMessageUnion::FleshlightLaunchFW12Cmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::FleshlightLaunchFW12Cmd)
        .then(|| ())
        .ok_or(ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::FleshlightLaunchFW12Cmd)),
      ButtplugDeviceCommandMessageUnion::KiirooCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::KiirooCmd)
        .then(|| ())
        .ok_or(ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::KiirooCmd)),
      ButtplugDeviceCommandMessageUnion::LinearCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::LinearCmd)
        .then(|| ())
        .ok_or(ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::LinearCmd)),
      ButtplugDeviceCommandMessageUnion::RawReadCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::RawReadCmd)
        .then(|| ())
        .ok_or(ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::RawReadCmd)),
      ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::RawSubscribeCmd)
        .then(|| ())
        .ok_or(ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::RawSubscribeCmd)),
      ButtplugDeviceCommandMessageUnion::RawUnsubscribeCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::RawUnsubscribeCmd)
        .then(|| ())
        .ok_or(ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::RawUnsubscribeCmd)),
      ButtplugDeviceCommandMessageUnion::RawWriteCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::RawWriteCmd)
        .then(|| ())
        .ok_or(ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::RawWriteCmd)),
      ButtplugDeviceCommandMessageUnion::RotateCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::RotateCmd)
        .then(|| ())
        .ok_or(ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::RotateCmd)),
      ButtplugDeviceCommandMessageUnion::RSSILevelCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::RSSILevelCmd)
        .then(|| ())
        .ok_or(ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::RSSILevelCmd)),
      ButtplugDeviceCommandMessageUnion::LevelCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::LevelCmd)
        .then(|| ())
        .ok_or(ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::LevelCmd)),
      // We translate SingleMotorVibrateCmd into Vibrate, so this one is special.
      ButtplugDeviceCommandMessageUnion::SingleMotorVibrateCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::VibrateCmd)
        .then(|| ())
        .ok_or(ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::VibrateCmd)),
      ButtplugDeviceCommandMessageUnion::StopDeviceCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::StopDeviceCmd)
        .then(|| ())
        .ok_or(ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::StopDeviceCmd)),
      ButtplugDeviceCommandMessageUnion::VibrateCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::VibrateCmd)
        .then(|| ())
        .ok_or(ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::VibrateCmd)),
      ButtplugDeviceCommandMessageUnion::VorzeA10CycloneCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::VorzeA10CycloneCmd)
        .then(|| ())
        .ok_or(ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::VorzeA10CycloneCmd)),
    }.map_err(|err| err.into())
  }
}

fn print_type_of<T>(_: &T) -> &'static str {
  std::any::type_name::<T>()
}

pub trait ButtplugProtocol: ButtplugProtocolProperties + ButtplugProtocolCommandHandler + Send + Sync {}

pub trait ButtplugProtocolCommandHandler: ButtplugProtocolProperties {
  // In order to not have to worry about id setting at the protocol level (this
  // should be taken care of in the server's device manager), we return server
  // messages but Buttplug errors.
  fn handle_command(
    &self,
    device: Arc<Hardware>,
    command_message: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugServerResultFuture {
    if let Err(err) = self.supports_message(&command_message) {
      return Box::pin(future::ready(Err(err)));
    }
    match command_message {
      ButtplugDeviceCommandMessageUnion::FleshlightLaunchFW12Cmd(msg) => {
        self.handle_fleshlight_launch_fw12_cmd(device, msg)
      }
      ButtplugDeviceCommandMessageUnion::KiirooCmd(msg) => self.handle_kiiroo_cmd(device, msg),
      ButtplugDeviceCommandMessageUnion::LevelCmd(msg) => self.handle_level_cmd(device, msg),
      ButtplugDeviceCommandMessageUnion::LinearCmd(msg) => self.handle_linear_cmd(device, msg),
      ButtplugDeviceCommandMessageUnion::RawReadCmd(msg) => self.handle_raw_read_cmd(device, msg),
      ButtplugDeviceCommandMessageUnion::RawWriteCmd(msg) => self.handle_raw_write_cmd(device, msg),
      ButtplugDeviceCommandMessageUnion::RotateCmd(msg) => self.handle_rotate_cmd(device, msg),
      ButtplugDeviceCommandMessageUnion::SingleMotorVibrateCmd(msg) => {
        self.handle_single_motor_vibrate_cmd(device, msg)
      }
      ButtplugDeviceCommandMessageUnion::StopDeviceCmd(msg) => {
        self.handle_stop_device_cmd(device, msg)
      }
      ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(msg) => {
        self.handle_raw_subscribe_cmd(device, msg)
      }
      ButtplugDeviceCommandMessageUnion::RawUnsubscribeCmd(msg) => {
        self.handle_raw_unsubscribe_cmd(device, msg)
      }
      ButtplugDeviceCommandMessageUnion::VibrateCmd(msg) => self.handle_vibrate_cmd(device, msg),
      ButtplugDeviceCommandMessageUnion::VorzeA10CycloneCmd(msg) => {
        self.handle_vorze_a10_cyclone_cmd(device, msg)
      }
      ButtplugDeviceCommandMessageUnion::BatteryLevelCmd(msg) => {
        self.handle_battery_level_cmd(device, msg)
      }
      ButtplugDeviceCommandMessageUnion::RSSILevelCmd(msg) => {
        self.handle_rssi_level_cmd(device, msg)
      }
    }
  }

  fn handle_stop_device_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::StopDeviceCmd,
  ) -> ButtplugServerResultFuture {
    let ok_return = messages::Ok::new(message.id());
    let fut_vec: Vec<ButtplugServerResultFuture> = self
      .stop_commands()
      .iter()
      .map(|cmd| self.handle_command(device.clone(), cmd.clone()))
      .collect();
    Box::pin(async move {
      // TODO We should be able to run these concurrently, and should return any error we get.
      for fut in fut_vec {
        if let Err(e) = fut.await {
          error!("{:?}", e);
        }
      }
      Ok(ok_return.into())
    })
  }

  fn handle_single_motor_vibrate_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::SingleMotorVibrateCmd,
  ) -> ButtplugServerResultFuture {
    // Time for sadness! In order to handle conversion of SingleMotorVibrateCmd, we need to know how
    // many vibrators a device has. We don't actually know that until we get to the protocol level,
    // so we're stuck parsing this here. Since we can assume SingleMotorVibrateCmd will ALWAYS map
    // to vibration, we can convert to VibrateCmd here and save ourselves having to handle it in
    // every protocol, meaning spec v0 and v1 programs will still be forward compatible with
    // vibrators.
    let vibrator_count;
    if let Some(attr) = self
      .device_attributes()
      .message_attributes(&ButtplugDeviceMessageType::VibrateCmd)
    {
      if let Some(count) = attr.feature_count() {
        vibrator_count = *count as usize;
      } else {
        return ButtplugDeviceError::ProtocolRequirementError(format!(
          "{} needs to support VibrateCmd with a feature count to use SingleMotorVibrateCmd.",
          self.name()
        ))
        .into();
      }
    } else {
      return ButtplugDeviceError::ProtocolRequirementError(format!(
        "{} needs to support VibrateCmd to use SingleMotorVibrateCmd.",
        self.name()
      ))
      .into();
    }
    let speed = message.speed();
    let mut cmds = vec![];
    for i in 0..vibrator_count {
      cmds.push(VibrateSubcommand::new(i as u32, speed));
    }
    let mut vibrate_cmd = VibrateCmd::new(message.device_index(), cmds);
    vibrate_cmd.set_id(message.id());
    self.handle_command(device, vibrate_cmd.into())
  }

  fn handle_raw_write_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::RawWriteCmd,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = device.write_value(message.into());
    Box::pin(async move { fut.await.map(|_| messages::Ok::new(id).into()) })
  }

  fn handle_raw_read_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::RawReadCmd,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = device.read_value(message.into());
    Box::pin(async move {
      fut.await.map(|mut msg| {
        msg.set_id(id);
        msg.into()
      })
    })
  }

  fn handle_raw_unsubscribe_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::RawUnsubscribeCmd,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = device.unsubscribe(message.into());
    Box::pin(async move { fut.await.map(|_| messages::Ok::new(id).into()) })
  }

  fn handle_raw_subscribe_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::RawSubscribeCmd,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = device.subscribe(message.into());
    Box::pin(async move { fut.await.map(|_| messages::Ok::new(id).into()) })
  }

  fn command_unimplemented(&self, command: &str) -> ButtplugServerResultFuture {
    #[cfg(build = "debug")]
    unimplemented!("Command not implemented for this protocol");
    #[cfg(not(build = "debug"))]
    Box::pin(future::ready(Err(
      ButtplugDeviceError::UnhandledCommand(format!(
        "Command not implemented for this protocol: {}",
        command
      ))
      .into(),
    )))
  }

  fn handle_level_cmd(
    &self,
    _device: Arc<Hardware>,
    message: messages::LevelCmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_vorze_a10_cyclone_cmd(
    &self,
    _device: Arc<Hardware>,
    message: messages::VorzeA10CycloneCmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_kiiroo_cmd(
    &self,
    _device: Arc<Hardware>,
    message: messages::KiirooCmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_fleshlight_launch_fw12_cmd(
    &self,
    _device: Arc<Hardware>,
    message: messages::FleshlightLaunchFW12Cmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_vibrate_cmd(
    &self,
    _device: Arc<Hardware>,
    message: messages::VibrateCmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_rotate_cmd(
    &self,
    _device: Arc<Hardware>,
    message: messages::RotateCmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_linear_cmd(
    &self,
    _device: Arc<Hardware>,
    message: messages::LinearCmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::BatteryLevelCmd,
  ) -> ButtplugServerResultFuture {
    // If we have a standardized BLE Battery endpoint, handle that above the
    // protocol, as it'll always be the same.
    if device.endpoints().contains(&Endpoint::RxBLEBattery) {
      info!("Trying to get battery reading.");
      let msg = HardwareReadCmd::new(Endpoint::RxBLEBattery, 1, 0);
      let fut = device.read_value(msg);
      Box::pin(async move {
        let raw_msg: RawReading = fut.await?;
        let battery_level = raw_msg.data()[0] as f64 / 100f64;
        let battery_reading =
          messages::BatteryLevelReading::new(message.device_index(), battery_level);
        info!("Got battery reading: {}", battery_level);
        Ok(battery_reading.into())
      })
    } else {
      self.command_unimplemented(print_type_of(&message))
    }
  }

  fn handle_rssi_level_cmd(
    &self,
    _device: Arc<Hardware>,
    message: messages::RSSILevelCmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented(print_type_of(&message))
  }
}

#[macro_export]
macro_rules! default_protocol_properties_definition {
  ( $protocol_name:ident ) => {
    impl ButtplugProtocolProperties for $protocol_name {
      fn name(&self) -> &str {
        self.device_attributes.name()
      }

      fn device_attributes(&self) -> &ProtocolDeviceAttributes {
        &self.device_attributes
      }

      fn stop_commands(&self) -> Vec<ButtplugDeviceCommandMessageUnion> {
        self.stop_commands.clone()
      }

      fn protocol_identifier(&self) -> &str {
        Self::PROTOCOL_IDENTIFIER
      }
    }
  };
}

#[macro_export]
macro_rules! default_protocol_definition {
  ( $protocol_name:ident, $protocol_identifier:tt ) => {
    pub struct $protocol_name {
      device_attributes: ProtocolDeviceAttributes,
      #[allow(dead_code)]
      manager: Arc<tokio::sync::Mutex<GenericCommandManager>>,
      stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
    }

    impl ButtplugProtocol for $protocol_name {}

    impl $protocol_name {
      const PROTOCOL_IDENTIFIER: &'static str = $protocol_identifier;

      pub fn new(device_attributes: ProtocolDeviceAttributes) -> Self
      where
        Self: Sized,
      {
        let manager = GenericCommandManager::new(&device_attributes);

        Self {
          device_attributes,
          stop_commands: manager.stop_commands(),
          manager: Arc::new(tokio::sync::Mutex::new(manager)),
        }
      }
    }

    crate::default_protocol_properties_definition!($protocol_name);
  };
}

#[macro_export]
macro_rules! default_protocol_trait_declaration {
  ( $protocol_name:ident ) => {
    paste::paste! {
      #[derive(Default, Debug)]
      pub struct [< $protocol_name Factory >] {}

      impl ButtplugProtocolFactory for [< $protocol_name Factory >] {
        fn try_create(
          &self,
          hardware: Arc<Hardware>,
          attributes_builder: ProtocolDeviceAttributesBuilder,
        ) -> futures::future::BoxFuture<
          'static,
          Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>,
        > {
          Box::pin(async move {
            let attributes = attributes_builder.create_from_hardware(&hardware)?;
            Ok(Box::new($protocol_name::new(attributes)) as Box<dyn ButtplugProtocol>)
          })
        }
  
        fn protocol_identifier(&self) -> &'static str {
          $protocol_name::PROTOCOL_IDENTIFIER
        }
      }
    }
  }
}

#[macro_export]
macro_rules! default_protocol_declaration {
  ( $protocol_name:ident, $protocol_identifier:tt ) => {
    crate::default_protocol_definition!($protocol_name, $protocol_identifier);

    crate::default_protocol_trait_declaration!($protocol_name);
  };
}

pub use default_protocol_declaration;
pub use default_protocol_definition;
pub use default_protocol_trait_declaration;
