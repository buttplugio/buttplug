// Since users can pick and choose protocols, we need all of these to be public.
pub mod aneros;
pub mod ankni;
pub mod buttplug_passthru;
pub mod cachito;
pub mod fleshlight_launch_helper;
pub mod fredorch;
pub mod generic_command_manager;
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

use super::DeviceImpl;
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::{
      self, ButtplugDeviceCommandMessageUnion, ButtplugDeviceMessage, ButtplugDeviceMessageType,
      ButtplugMessage, RawReading, VibrateCmd, VibrateSubcommand,
    },
  },
  device::{
    configuration_manager::{DeviceAttributesBuilder, ProtocolDeviceAttributes},
    ButtplugDeviceResultFuture, DeviceReadCmd, Endpoint,
  },
};
use dashmap::DashMap;
use futures::future::{self, BoxFuture};
use generic_command_manager::GenericCommandManager;
use std::sync::Arc;

pub type TryCreateProtocolFunc =
  fn(
    Arc<DeviceImpl>,
    DeviceAttributesBuilder,
  ) -> BoxFuture<'static, Result<Box<dyn ButtplugProtocol>, ButtplugError>>;

pub fn add_to_protocol_map<T>(map: &DashMap<String, TryCreateProtocolFunc>, protocol_name: &str)
where
  T: ButtplugProtocol,
{
  map.insert(
    protocol_name.to_owned(),
    T::try_create as TryCreateProtocolFunc,
  );
}

pub fn get_default_protocol_map() -> DashMap<String, TryCreateProtocolFunc> {
  let map = DashMap::new();
  add_to_protocol_map::<aneros::Aneros>(&map, "aneros");
  add_to_protocol_map::<ankni::Ankni>(&map, "ankni");
  add_to_protocol_map::<buttplug_passthru::ButtplugPassthru>(&map, "buttplug-passthru");
  add_to_protocol_map::<cachito::Cachito>(&map, "cachito");
  add_to_protocol_map::<fredorch::Fredorch>(&map, "fredorch");
  add_to_protocol_map::<hismith::Hismith>(&map, "hismith");
  add_to_protocol_map::<hgod::Hgod>(&map, "hgod");
  add_to_protocol_map::<htk_bm::HtkBm>(&map, "htk_bm");
  add_to_protocol_map::<jejoue::JeJoue>(&map, "jejoue");
  add_to_protocol_map::<kiiroo_v2::KiirooV2>(&map, "kiiroo-v2");
  add_to_protocol_map::<kiiroo_v2_vibrator::KiirooV2Vibrator>(&map, "kiiroo-v2-vibrator");
  add_to_protocol_map::<kiiroo_v21::KiirooV21>(&map, "kiiroo-v21");
  add_to_protocol_map::<kiiroo_v21_initialized::KiirooV21Initialized>(
    &map,
    "kiiroo-v21-initialized",
  );
  add_to_protocol_map::<lelof1s::LeloF1s>(&map, "lelo-f1s");
  add_to_protocol_map::<lelof1sv2::LeloF1sV2>(&map, "lelo-f1sv2");
  add_to_protocol_map::<libo_elle::LiboElle>(&map, "libo-elle");
  add_to_protocol_map::<libo_shark::LiboShark>(&map, "libo-shark");
  add_to_protocol_map::<libo_vibes::LiboVibes>(&map, "libo-vibes");
  add_to_protocol_map::<lovehoney_desire::LovehoneyDesire>(&map, "lovehoney-desire");
  add_to_protocol_map::<lovedistance::LoveDistance>(&map, "lovedistance");
  add_to_protocol_map::<lovense::Lovense>(&map, "lovense");
  add_to_protocol_map::<lovense_connect_service::LovenseConnectService>(
    &map,
    "lovense-connect-service",
  );
  add_to_protocol_map::<lovenuts::LoveNuts>(&map, "lovenuts");
  add_to_protocol_map::<magic_motion_v1::MagicMotionV1>(&map, "magic-motion-1");
  add_to_protocol_map::<magic_motion_v2::MagicMotionV2>(&map, "magic-motion-2");
  add_to_protocol_map::<magic_motion_v3::MagicMotionV3>(&map, "magic-motion-3");
  add_to_protocol_map::<magic_motion_v4::MagicMotionV4>(&map, "magic-motion-4");
  add_to_protocol_map::<mannuo::ManNuo>(&map, "mannuo");
  add_to_protocol_map::<maxpro::Maxpro>(&map, "maxpro");
  add_to_protocol_map::<mizzzee::MizzZee>(&map, "mizzzee");
  add_to_protocol_map::<motorbunny::Motorbunny>(&map, "motorbunny");
  add_to_protocol_map::<mysteryvibe::MysteryVibe>(&map, "mysteryvibe");
  add_to_protocol_map::<nobra::Nobra>(&map, "nobra");
  add_to_protocol_map::<patoo::Patoo>(&map, "patoo");
  add_to_protocol_map::<picobong::Picobong>(&map, "picobong");
  add_to_protocol_map::<prettylove::PrettyLove>(&map, "prettylove");
  add_to_protocol_map::<raw_protocol::RawProtocol>(&map, "raw");
  add_to_protocol_map::<realov::Realov>(&map, "realov");
  add_to_protocol_map::<satisfyer::Satisfyer>(&map, "satisfyer");
  add_to_protocol_map::<svakom::Svakom>(&map, "svakom");
  add_to_protocol_map::<svakom_alex::SvakomAlex>(&map, "svakom-alex");
  add_to_protocol_map::<svakom_iker::SvakomIker>(&map, "svakom-iker");
  add_to_protocol_map::<svakom_sam::SvakomSam>(&map, "svakom-sam");
  add_to_protocol_map::<tcode_v03::TCodeV03>(&map, "tcode-v03");
  add_to_protocol_map::<thehandy::TheHandy>(&map, "thehandy");
  add_to_protocol_map::<vibratissimo::Vibratissimo>(&map, "vibratissimo");
  add_to_protocol_map::<vorze_sa::VorzeSA>(&map, "vorze-sa");
  add_to_protocol_map::<wevibe::WeVibe>(&map, "wevibe");
  add_to_protocol_map::<wevibe8bit::WeVibe8Bit>(&map, "wevibe-8bit");
  add_to_protocol_map::<xinput::XInput>(&map, "xinput");
  add_to_protocol_map::<youcups::Youcups>(&map, "youcups");
  add_to_protocol_map::<youou::Youou>(&map, "youou");
  add_to_protocol_map::<zalo::Zalo>(&map, "zalo");
  map
}

pub trait ButtplugProtocol: ButtplugProtocolCommandHandler + Sync {
  fn try_create(
    device_impl: Arc<DeviceImpl>,
    attributes_builder: DeviceAttributesBuilder,
  ) -> BoxFuture<'static, Result<Box<dyn ButtplugProtocol>, ButtplugError>>
  where
    Self: Sized;
}

pub trait ButtplugProtocolProperties {
  fn name(&self) -> &str;
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
        .allows_message(&ButtplugDeviceMessageType::BatteryLevelCmd),
      ButtplugDeviceCommandMessageUnion::FleshlightLaunchFW12Cmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::FleshlightLaunchFW12Cmd),
      ButtplugDeviceCommandMessageUnion::KiirooCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::KiirooCmd),
      ButtplugDeviceCommandMessageUnion::LinearCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::LinearCmd),
      ButtplugDeviceCommandMessageUnion::RawReadCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::RawReadCmd),
      ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::RawSubscribeCmd),
      ButtplugDeviceCommandMessageUnion::RawUnsubscribeCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::RawUnsubscribeCmd),
      ButtplugDeviceCommandMessageUnion::RawWriteCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::RawWriteCmd),
      ButtplugDeviceCommandMessageUnion::RotateCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::RotateCmd),
      ButtplugDeviceCommandMessageUnion::RSSILevelCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::RSSILevelCmd),
      // We translate SingleMotorVibrateCmd into Vibrate, so this one is special.
      ButtplugDeviceCommandMessageUnion::SingleMotorVibrateCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::VibrateCmd),
      ButtplugDeviceCommandMessageUnion::StopDeviceCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::StopDeviceCmd),
      ButtplugDeviceCommandMessageUnion::VibrateCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::VibrateCmd),
      ButtplugDeviceCommandMessageUnion::VorzeA10CycloneCmd(_) => self
        .device_attributes()
        .allows_message(&ButtplugDeviceMessageType::VorzeA10CycloneCmd),
    }.map_err(|err| err.into())
  }
}

fn print_type_of<T>(_: &T) -> &'static str {
  std::any::type_name::<T>()
}

pub trait ButtplugProtocolCommandHandler: Send + ButtplugProtocolProperties {
  // In order to not have to worry about id setting at the protocol level (this
  // should be taken care of in the server's device manager), we return server
  // messages but Buttplug errors.
  fn handle_command(
    &self,
    device: Arc<DeviceImpl>,
    command_message: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugDeviceResultFuture {
    if let Err(err) = self.supports_message(&command_message) {
      return Box::pin(future::ready(Err(err)));
    }
    match command_message {
      ButtplugDeviceCommandMessageUnion::FleshlightLaunchFW12Cmd(msg) => {
        self.handle_fleshlight_launch_fw12_cmd(device, msg)
      }
      ButtplugDeviceCommandMessageUnion::KiirooCmd(msg) => self.handle_kiiroo_cmd(device, msg),
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
    device: Arc<DeviceImpl>,
    message: messages::StopDeviceCmd,
  ) -> ButtplugDeviceResultFuture {
    let ok_return = messages::Ok::new(message.id());
    let fut_vec: Vec<ButtplugDeviceResultFuture> = self
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
    device: Arc<DeviceImpl>,
    message: messages::SingleMotorVibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // Time for sadness! In order to handle conversion of
    // SingleMotorVibrateCmd, we need to know how many vibrators a device
    // has. We don't actually know that until we get to the protocol level,
    // so we're stuck parsing this here. Since we can assume
    // SingleMotorVibrateCmd will ALWAYS map to vibration, we can convert to
    // VibrateCmd here and save ourselves having to handle it in every
    // protocol, meaning spec v0 and v1 programs will still be forward
    // compatible with vibrators.
    let vibrator_count;
    if let Some(attr) = self
      .device_attributes()
      .message_attributes(&ButtplugDeviceMessageType::VibrateCmd)
    {
      if let Some(count) = attr.feature_count {
        vibrator_count = count as usize;
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
    device: Arc<DeviceImpl>,
    message: messages::RawWriteCmd,
  ) -> ButtplugDeviceResultFuture {
    let id = message.id();
    let fut = device.write_value(message.into());
    Box::pin(async move { fut.await.map(|_| messages::Ok::new(id).into()) })
  }

  fn handle_raw_read_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::RawReadCmd,
  ) -> ButtplugDeviceResultFuture {
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
    device: Arc<DeviceImpl>,
    message: messages::RawUnsubscribeCmd,
  ) -> ButtplugDeviceResultFuture {
    let id = message.id();
    let fut = device.unsubscribe(message.into());
    Box::pin(async move { fut.await.map(|_| messages::Ok::new(id).into()) })
  }

  fn handle_raw_subscribe_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::RawSubscribeCmd,
  ) -> ButtplugDeviceResultFuture {
    let id = message.id();
    let fut = device.subscribe(message.into());
    Box::pin(async move { fut.await.map(|_| messages::Ok::new(id).into()) })
  }

  fn command_unimplemented(&self, command: &str) -> ButtplugDeviceResultFuture {
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

  fn handle_vorze_a10_cyclone_cmd(
    &self,
    _device: Arc<DeviceImpl>,
    message: messages::VorzeA10CycloneCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_kiiroo_cmd(
    &self,
    _device: Arc<DeviceImpl>,
    message: messages::KiirooCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_fleshlight_launch_fw12_cmd(
    &self,
    _device: Arc<DeviceImpl>,
    message: messages::FleshlightLaunchFW12Cmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_vibrate_cmd(
    &self,
    _device: Arc<DeviceImpl>,
    message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_rotate_cmd(
    &self,
    _device: Arc<DeviceImpl>,
    message: messages::RotateCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_linear_cmd(
    &self,
    _device: Arc<DeviceImpl>,
    message: messages::LinearCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented(print_type_of(&message))
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::BatteryLevelCmd,
  ) -> ButtplugDeviceResultFuture {
    // If we have a standardized BLE Battery endpoint, handle that above the
    // protocol, as it'll always be the same.
    if device.endpoints().contains(&Endpoint::RxBLEBattery) {
      info!("Trying to get battery reading.");
      let msg = DeviceReadCmd::new(Endpoint::RxBLEBattery, 1, 0);
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
    _device: Arc<DeviceImpl>,
    message: messages::RSSILevelCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented(print_type_of(&message))
  }
}

#[macro_export]
macro_rules! default_protocol_definition {
  ( $protocol_name:ident ) => {
    #[derive(ButtplugProtocolProperties)]
    pub struct $protocol_name {
      device_attributes: ProtocolDeviceAttributes,
      #[allow(dead_code)]
      manager: Arc<tokio::sync::Mutex<GenericCommandManager>>,
      stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
    }

    impl $protocol_name {
      pub fn new(device_attributes: ProtocolDeviceAttributes) -> Self
      where
        Self: Sized,
      {
        let manager = GenericCommandManager::new(&device_attributes);

        Self {
          device_attributes,
          stop_commands: manager.get_stop_commands(),
          manager: Arc::new(tokio::sync::Mutex::new(manager)),
        }
      }
    }
  };
}

#[macro_export]
macro_rules! default_protocol_trait_declaration {
  ( $protocol_name:ident ) => {
    impl ButtplugProtocol for $protocol_name {
      fn try_create(
        device_impl: Arc<crate::device::DeviceImpl>,
        attributes_builder: DeviceAttributesBuilder,
      ) -> futures::future::BoxFuture<
        'static,
        Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>,
      > {
        Box::pin(async move {
          let attributes = attributes_builder.create_from_impl(&device_impl)?;
          Ok(Box::new(Self::new(attributes)) as Box<dyn ButtplugProtocol>)
        })
      }
    }
  };
}

#[macro_export]
macro_rules! default_protocol_declaration {
  ( $protocol_name:ident ) => {
    crate::default_protocol_definition!($protocol_name);

    crate::default_protocol_trait_declaration!($protocol_name);
  };
}

pub use default_protocol_declaration;
pub use default_protocol_definition;
pub use default_protocol_trait_declaration;
