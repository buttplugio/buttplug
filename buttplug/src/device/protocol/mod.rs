mod aneros;
mod fleshlight_launch_helper;
mod generic_command_manager;
mod kiiroo_v2;
mod kiiroo_v21;
mod kiiroo_v2_vibrator;
mod lelof1s;
mod libo_elle;
mod libo_shark;
mod libo_vibes;
mod lovehoney_desire;
mod lovense;
mod magic_motion_v1;
mod magic_motion_v2;
mod magic_motion_v3;
mod maxpro;
mod motorbunny;
mod mysteryvibe;
mod nobra;
mod picobong;
mod prettylove;
mod raw_protocol;
mod realov;
mod svakom;
mod thehandy;
mod vibratissimo;
mod vorze_sa;
mod wevibe;
mod wevibe8bit;
mod xinput;
mod youcups;
mod youou;

use super::DeviceImpl;
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::{
      self,
      ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceMessage,
      ButtplugDeviceMessageType,
      ButtplugMessage,
      DeviceMessageAttributesMap,
      RawReading,
      VibrateCmd,
      VibrateSubcommand,
    },
  },
  device::{
    configuration_manager::DeviceProtocolConfiguration,
    ButtplugDeviceResultFuture,
    DeviceReadCmd,
    Endpoint,
  },
};
use futures::future::{self, BoxFuture};
use std::convert::TryFrom;
use std::sync::Arc;

pub enum ProtocolTypes {
  Aneros,
  KiirooV2,
  KiirooV2Vibrator,
  KiirooV21,
  LeloF1s,
  LiboElle,
  LiboShark,
  LiboVibes,
  LovehoneyDesire,
  Lovense,
  MagicMotionV1,
  MagicMotionV2,
  MagicMotionV3,
  Maxpro,
  Motorbunny,
  MysteryVibe,
  Nobra,
  Picobong,
  PrettyLove,
  RawProtocol,
  Realov,
  Svakom,
  TheHandy,
  Vibratissimo,
  VorzeSA,
  WeVibe,
  WeVibe8Bit,
  XInput,
  Youcups,
  Youou,
}

impl TryFrom<&str> for ProtocolTypes {
  type Error = ButtplugError;

  fn try_from(protocol_name: &str) -> Result<Self, Self::Error> {
    match protocol_name {
      "aneros" => Ok(ProtocolTypes::Aneros),
      "kiiroo-v2" => Ok(ProtocolTypes::KiirooV2),
      "kiiroo-v2-vibrator" => Ok(ProtocolTypes::KiirooV2Vibrator),
      "kiiroo-v21" => Ok(ProtocolTypes::KiirooV21),
      "lelo-f1s" => Ok(ProtocolTypes::LeloF1s),
      "libo-elle" => Ok(ProtocolTypes::LiboElle),
      "libo-shark" => Ok(ProtocolTypes::LiboShark),
      "libo-vibes" => Ok(ProtocolTypes::LiboVibes),
      "lovehoney-desire" => Ok(ProtocolTypes::LovehoneyDesire),
      "lovense" => Ok(ProtocolTypes::Lovense),
      "magic-motion-1" => Ok(ProtocolTypes::MagicMotionV1),
      "magic-motion-2" => Ok(ProtocolTypes::MagicMotionV2),
      "magic-motion-3" => Ok(ProtocolTypes::MagicMotionV3),
      "maxpro" => Ok(ProtocolTypes::Maxpro),
      "motorbunny" => Ok(ProtocolTypes::Motorbunny),
      "mysteryvibe" => Ok(ProtocolTypes::MysteryVibe),
      "nobra" => Ok(ProtocolTypes::Nobra),
      "picobong" => Ok(ProtocolTypes::Picobong),
      "prettylove" => Ok(ProtocolTypes::PrettyLove),
      "raw" => Ok(ProtocolTypes::RawProtocol),
      "realov" => Ok(ProtocolTypes::Realov),
      "svakom" => Ok(ProtocolTypes::Svakom),
      "thehandy" => Ok(ProtocolTypes::TheHandy),
      "vibratissimo" => Ok(ProtocolTypes::Vibratissimo),
      "vorze-sa" => Ok(ProtocolTypes::VorzeSA),
      "wevibe" => Ok(ProtocolTypes::WeVibe),
      "wevibe-8bit" => Ok(ProtocolTypes::WeVibe8Bit),
      "xinput" => Ok(ProtocolTypes::XInput),
      "youcups" => Ok(ProtocolTypes::Youcups),
      "youou" => Ok(ProtocolTypes::Youou),
      _ => {
        error!("Protocol {} not implemented.", protocol_name);
        Err(ButtplugDeviceError::ProtocolNotImplemented(protocol_name.to_owned()).into())
      }
    }
  }
}

pub fn try_create_protocol(
  protocol_type: &ProtocolTypes,
  device: Arc<DeviceImpl>,
  config: DeviceProtocolConfiguration,
) -> BoxFuture<'static, Result<Box<dyn ButtplugProtocol>, ButtplugError>> {
  match protocol_type {
    ProtocolTypes::Aneros => aneros::Aneros::try_create(device, config),
    ProtocolTypes::KiirooV2 => kiiroo_v2::KiirooV2::try_create(device, config),
    ProtocolTypes::KiirooV2Vibrator => {
      kiiroo_v2_vibrator::KiirooV2Vibrator::try_create(device, config)
    }
    ProtocolTypes::KiirooV21 => kiiroo_v21::KiirooV21::try_create(device, config),
    ProtocolTypes::LeloF1s => lelof1s::LeloF1s::try_create(device, config),
    ProtocolTypes::LiboElle => libo_elle::LiboElle::try_create(device, config),
    ProtocolTypes::LiboShark => libo_shark::LiboShark::try_create(device, config),
    ProtocolTypes::LiboVibes => libo_vibes::LiboVibes::try_create(device, config),
    ProtocolTypes::LovehoneyDesire => lovehoney_desire::LovehoneyDesire::try_create(device, config),
    ProtocolTypes::Lovense => lovense::Lovense::try_create(device, config),
    ProtocolTypes::MagicMotionV1 => magic_motion_v1::MagicMotionV1::try_create(device, config),
    ProtocolTypes::MagicMotionV2 => magic_motion_v2::MagicMotionV2::try_create(device, config),
    ProtocolTypes::MagicMotionV3 => magic_motion_v3::MagicMotionV3::try_create(device, config),
    ProtocolTypes::Maxpro => maxpro::Maxpro::try_create(device, config),
    ProtocolTypes::Motorbunny => motorbunny::Motorbunny::try_create(device, config),
    ProtocolTypes::MysteryVibe => mysteryvibe::MysteryVibe::try_create(device, config),
    ProtocolTypes::Nobra => nobra::Nobra::try_create(device, config),
    ProtocolTypes::Picobong => picobong::Picobong::try_create(device, config),
    ProtocolTypes::PrettyLove => prettylove::PrettyLove::try_create(device, config),
    ProtocolTypes::RawProtocol => raw_protocol::RawProtocol::try_create(device, config),
    ProtocolTypes::Realov => realov::Realov::try_create(device, config),
    ProtocolTypes::Svakom => svakom::Svakom::try_create(device, config),
    ProtocolTypes::TheHandy => thehandy::TheHandy::try_create(device, config),
    ProtocolTypes::Vibratissimo => vibratissimo::Vibratissimo::try_create(device, config),
    ProtocolTypes::VorzeSA => vorze_sa::VorzeSA::try_create(device, config),
    ProtocolTypes::WeVibe => wevibe::WeVibe::try_create(device, config),
    ProtocolTypes::WeVibe8Bit => wevibe8bit::WeVibe8Bit::try_create(device, config),
    ProtocolTypes::XInput => xinput::XInput::try_create(device, config),
    ProtocolTypes::Youcups => youcups::Youcups::try_create(device, config),
    ProtocolTypes::Youou => youou::Youou::try_create(device, config),
  }
}

pub trait ButtplugProtocol: ButtplugProtocolCommandHandler + Sync {
  fn try_create(
    device_impl: Arc<DeviceImpl>,
    config: DeviceProtocolConfiguration,
  ) -> BoxFuture<'static, Result<Box<dyn ButtplugProtocol>, ButtplugError>>
  where
    Self: Sized,
  {
    let endpoints = device_impl.endpoints();
    let name = device_impl.name().to_owned();
    let init_fut = Self::initialize(device_impl);
    Box::pin(async move {
      let device_identifier = match init_fut.await {
        Ok(maybe_ident) => maybe_ident.unwrap_or(name),
        Err(err) => return Err(err),
      };
      let (names, attrs) = config.get_attributes(&device_identifier, &endpoints)?;
      let name = names.get("en-us").unwrap().clone();
      Ok(Self::new_protocol(&name, attrs))
    })
  }

  fn initialize(
    _device_impl: Arc<DeviceImpl>,
  ) -> BoxFuture<'static, Result<Option<String>, ButtplugError>>
  where
    Self: Sized,
  {
    Box::pin(future::ready(Ok(None)))
  }

  fn new_protocol(name: &str, attrs: DeviceMessageAttributesMap) -> Box<dyn ButtplugProtocol>
  where
    Self: Sized;
}

fn check_message_support(
  message_type: &ButtplugDeviceMessageType,
  message_attributes: &DeviceMessageAttributesMap,
) -> Result<(), ButtplugError> {
  if !message_attributes.contains_key(message_type) {
    Err(ButtplugDeviceError::MessageNotSupported(*message_type).into())
  } else {
    Ok(())
  }
}

pub trait ButtplugProtocolProperties {
  fn name(&self) -> &str;
  fn message_attributes(&self) -> DeviceMessageAttributesMap;
  fn stop_commands(&self) -> Vec<ButtplugDeviceCommandMessageUnion>;

  fn supports_message(
    &self,
    message: &ButtplugDeviceCommandMessageUnion,
  ) -> Result<(), ButtplugError> {
    // TODO This should be generated by a macro, as should the types enum.
    match message {
      ButtplugDeviceCommandMessageUnion::BatteryLevelCmd(_) => check_message_support(
        &ButtplugDeviceMessageType::BatteryLevelCmd,
        &self.message_attributes(),
      ),
      ButtplugDeviceCommandMessageUnion::FleshlightLaunchFW12Cmd(_) => check_message_support(
        &ButtplugDeviceMessageType::FleshlightLaunchFW12Cmd,
        &self.message_attributes(),
      ),
      ButtplugDeviceCommandMessageUnion::KiirooCmd(_) => check_message_support(
        &ButtplugDeviceMessageType::KiirooCmd,
        &self.message_attributes(),
      ),
      ButtplugDeviceCommandMessageUnion::LinearCmd(_) => check_message_support(
        &ButtplugDeviceMessageType::LinearCmd,
        &self.message_attributes(),
      ),
      ButtplugDeviceCommandMessageUnion::RawReadCmd(_) => check_message_support(
        &ButtplugDeviceMessageType::RawReadCmd,
        &self.message_attributes(),
      ),
      ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(_) => check_message_support(
        &ButtplugDeviceMessageType::RawSubscribeCmd,
        &self.message_attributes(),
      ),
      ButtplugDeviceCommandMessageUnion::RawUnsubscribeCmd(_) => check_message_support(
        &ButtplugDeviceMessageType::RawUnsubscribeCmd,
        &self.message_attributes(),
      ),
      ButtplugDeviceCommandMessageUnion::RawWriteCmd(_) => check_message_support(
        &ButtplugDeviceMessageType::RawWriteCmd,
        &self.message_attributes(),
      ),
      ButtplugDeviceCommandMessageUnion::RotateCmd(_) => check_message_support(
        &ButtplugDeviceMessageType::RotateCmd,
        &self.message_attributes(),
      ),
      ButtplugDeviceCommandMessageUnion::RSSILevelCmd(_) => check_message_support(
        &ButtplugDeviceMessageType::RSSILevelCmd,
        &self.message_attributes(),
      ),
      // We translate SingleMotorVibrateCmd into Vibrate, so this one is special.
      ButtplugDeviceCommandMessageUnion::SingleMotorVibrateCmd(_) => check_message_support(
        &ButtplugDeviceMessageType::VibrateCmd,
        &self.message_attributes(),
      ),
      ButtplugDeviceCommandMessageUnion::StopDeviceCmd(_) => check_message_support(
        &ButtplugDeviceMessageType::StopDeviceCmd,
        &self.message_attributes(),
      ),
      ButtplugDeviceCommandMessageUnion::VibrateCmd(_) => check_message_support(
        &ButtplugDeviceMessageType::VibrateCmd,
        &self.message_attributes(),
      ),
      ButtplugDeviceCommandMessageUnion::VorzeA10CycloneCmd(_) => check_message_support(
        &ButtplugDeviceMessageType::VorzeA10CycloneCmd,
        &self.message_attributes(),
      ),
    }
  }
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
      .message_attributes()
      .get(&ButtplugDeviceMessageType::VibrateCmd)
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

  fn command_unimplemented(&self) -> ButtplugDeviceResultFuture {
    #[cfg(build = "debug")]
    unimplemented!("Command not implemented for this protocol");
    #[cfg(not(build = "debug"))]
    Box::pin(future::ready(Err(
      ButtplugDeviceError::UnhandledCommand("Command not implemented for this protocol".to_owned())
        .into(),
    )))
  }

  fn handle_vorze_a10_cyclone_cmd(
    &self,
    _device: Arc<DeviceImpl>,
    _message: messages::VorzeA10CycloneCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented()
  }

  fn handle_kiiroo_cmd(
    &self,
    _device: Arc<DeviceImpl>,
    _message: messages::KiirooCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented()
  }

  fn handle_fleshlight_launch_fw12_cmd(
    &self,
    _device: Arc<DeviceImpl>,
    _message: messages::FleshlightLaunchFW12Cmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented()
  }

  fn handle_vibrate_cmd(
    &self,
    _device: Arc<DeviceImpl>,
    _message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented()
  }

  fn handle_rotate_cmd(
    &self,
    _device: Arc<DeviceImpl>,
    _message: messages::RotateCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented()
  }

  fn handle_linear_cmd(
    &self,
    _device: Arc<DeviceImpl>,
    _message: messages::LinearCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented()
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
      self.command_unimplemented()
    }
  }

  fn handle_rssi_level_cmd(
    &self,
    _device: Arc<DeviceImpl>,
    _message: messages::RSSILevelCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented()
  }
}
