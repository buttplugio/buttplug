mod aneros;
mod fleshlight_launch_helper;
mod generic_command_manager;
mod kiiroov2;
mod lelof1s;
mod lovehoney_desire;
mod lovense;
mod maxpro;
mod motorbunny;
mod picobong;
mod prettylove;
mod raw_protocol;
mod realov;
mod svakom;
mod vibratissimo;
mod vorze_sa;
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
      ButtplugDeviceMessageType,
      ButtplugMessage,
      MessageAttributesMap,
      VibrateCmd,
      VibrateSubcommand,
    },
  },
  device::{configuration_manager::DeviceProtocolConfiguration, ButtplugDeviceResultFuture},
};
use futures::future::{self, BoxFuture};
use std::convert::TryFrom;
use std::sync::Arc;

pub enum ProtocolTypes {
  Aneros,
  KiirooV2,
  LeloF1s,
  LovehoneyDesire,
  Lovense,
  Maxpro,
  Motorbunny,
  Picobong,
  PrettyLove,
  RawProtocol,
  Realov,
  Svakom,
  Vibratissimo,
  VorzeSA,
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
      "lelo-f1s" => Ok(ProtocolTypes::LeloF1s),
      "lovehoney-desire" => Ok(ProtocolTypes::LovehoneyDesire),      
      "lovense" => Ok(ProtocolTypes::Lovense),
      "maxpro" => Ok(ProtocolTypes::Maxpro),
      "motorbunny" => Ok(ProtocolTypes::Motorbunny),
      "picobong" => Ok(ProtocolTypes::Picobong),
      "prettylove" => Ok(ProtocolTypes::PrettyLove),
      "raw" => Ok(ProtocolTypes::RawProtocol),      
      "realov" => Ok(ProtocolTypes::Realov),
      "svakom" => Ok(ProtocolTypes::Svakom),
      "vibratissimo" => Ok(ProtocolTypes::Vibratissimo),
      "vorze-sa" => Ok(ProtocolTypes::VorzeSA),
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
  device: &dyn DeviceImpl,
  config: DeviceProtocolConfiguration,
) -> BoxFuture<'static, Result<Box<dyn ButtplugProtocol>, ButtplugError>> {
  match protocol_type {
    ProtocolTypes::Aneros => aneros::Aneros::try_create(device, config),
    ProtocolTypes::KiirooV2 => kiiroov2::KiirooV2::try_create(device, config),
    ProtocolTypes::LeloF1s => lelof1s::LeloF1s::try_create(device, config),
    ProtocolTypes::LovehoneyDesire => lovehoney_desire::LovehoneyDesire::try_create(device, config),
    ProtocolTypes::Lovense => lovense::Lovense::try_create(device, config),
    ProtocolTypes::Maxpro => maxpro::Maxpro::try_create(device, config),
    ProtocolTypes::Motorbunny => motorbunny::Motorbunny::try_create(device, config),
    ProtocolTypes::Picobong => picobong::Picobong::try_create(device, config),
    ProtocolTypes::PrettyLove => prettylove::PrettyLove::try_create(device, config),
    ProtocolTypes::RawProtocol => raw_protocol::RawProtocol::try_create(device, config),
    ProtocolTypes::Realov => realov::Realov::try_create(device, config),
    ProtocolTypes::Svakom => svakom::Svakom::try_create(device, config),
    ProtocolTypes::Vibratissimo => vibratissimo::Vibratissimo::try_create(device, config),
    ProtocolTypes::VorzeSA => vorze_sa::VorzeSA::try_create(device, config),
    ProtocolTypes::XInput => xinput::XInput::try_create(device, config),
    ProtocolTypes::Youcups => youcups::Youcups::try_create(device, config),
    ProtocolTypes::Youou => youou::Youou::try_create(device, config),
  }
}

pub trait ButtplugProtocolCreator: ButtplugProtocol {
  fn try_create(
    device_impl: &dyn DeviceImpl,
    config: DeviceProtocolConfiguration,
  ) -> BoxFuture<'static, Result<Box<dyn ButtplugProtocol>, ButtplugError>>
  where
    Self: Sized,
  {
    let (names, attrs) = config.get_attributes(device_impl.name()).unwrap();
    let name = names.get("en-us").unwrap().clone();
    Box::pin(async move { Ok(Self::new_protocol(&name, attrs)) })
  }

  fn new_protocol(name: &str, attrs: MessageAttributesMap) -> Box<dyn ButtplugProtocol>
  where
    Self: Sized;
}

pub trait ButtplugProtocol: ButtplugProtocolCommandHandler + Sync {}

pub trait ButtplugProtocolProperties {
  fn name(&self) -> &str;
  fn message_attributes(&self) -> MessageAttributesMap;
  fn stop_commands(&self) -> Vec<ButtplugDeviceCommandMessageUnion>;
}

pub trait ButtplugProtocolCommandHandler: Send + ButtplugProtocolProperties {
  // In order to not have to worry about id setting at the protocol level (this
  // should be taken care of in the server's device manager), we return server
  // messages but Buttplug errors.
  fn handle_command(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
    command_message: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugDeviceResultFuture {
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
    }
  }

  fn handle_stop_device_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
    message: messages::StopDeviceCmd,
  ) -> ButtplugDeviceResultFuture {
    let ok_return = messages::Ok::new(message.get_id());
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
    device: Arc<Box<dyn DeviceImpl>>,
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
    let speed = message.speed;
    let mut cmds = vec![];
    for i in 0..vibrator_count {
      cmds.push(VibrateSubcommand::new(i as u32, speed));
    }
    let mut vibrate_cmd = VibrateCmd::new(message.device_index, cmds);
    vibrate_cmd.set_id(message.get_id());
    self.handle_command(device, vibrate_cmd.into())
  }

  fn handle_raw_write_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
    message: messages::RawWriteCmd,
  ) -> ButtplugDeviceResultFuture {
    let id = message.get_id();
    let fut = device.write_value(message.into());
    Box::pin(async move { fut.await.map(|_| messages::Ok::new(id).into()) })
  }

  fn handle_raw_read_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
    message: messages::RawReadCmd,
  ) -> ButtplugDeviceResultFuture {
    let id = message.get_id();
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
    device: Arc<Box<dyn DeviceImpl>>,
    message: messages::RawUnsubscribeCmd,
  ) -> ButtplugDeviceResultFuture {
    let id = message.get_id();
    let fut = device.unsubscribe(message.into());
    Box::pin(async move { fut.await.map(|_| messages::Ok::new(id).into()) })
  }

  fn handle_raw_subscribe_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
    message: messages::RawSubscribeCmd,
  ) -> ButtplugDeviceResultFuture {
    let id = message.get_id();
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
    _device: Arc<Box<dyn DeviceImpl>>,
    _message: messages::VorzeA10CycloneCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented()
  }

  fn handle_kiiroo_cmd(
    &self,
    _device: Arc<Box<dyn DeviceImpl>>,
    _message: messages::KiirooCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented()
  }

  fn handle_fleshlight_launch_fw12_cmd(
    &self,
    _device: Arc<Box<dyn DeviceImpl>>,
    _message: messages::FleshlightLaunchFW12Cmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented()
  }

  fn handle_vibrate_cmd(
    &self,
    _device: Arc<Box<dyn DeviceImpl>>,
    _message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented()
  }

  fn handle_rotate_cmd(
    &self,
    _device: Arc<Box<dyn DeviceImpl>>,
    _message: messages::RotateCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented()
  }

  fn handle_linear_cmd(
    &self,
    _device: Arc<Box<dyn DeviceImpl>>,
    _message: messages::LinearCmd,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented()
  }
  /*
  fn handle_battery_level_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
    message: messages::Battery,
  ) -> ButtplugDeviceResultFuture {
    self.command_unimplemented()
  }

  fn handle_rssi_level_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
    message: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugDeviceResultFuture {
    unimplemented!("Command not implemented for this protocol");
  }
  */
}
