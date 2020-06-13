mod generic_command_manager;
mod aneros;
mod lovehoney_desire;
mod lovense;
mod maxpro;
mod picobong;
mod prettylove;
mod realov;
mod svakom;
mod vorze_sa;
mod xinput;
mod youcups;
mod youou;

use super::DeviceImpl;
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::{self, ButtplugDeviceCommandMessageUnion, ButtplugMessage, MessageAttributesMap, ButtplugDeviceMessageType, VibrateCmd, VibrateSubcommand},
  },
  device::configuration_manager::{DeviceProtocolConfiguration, ProtocolConstructor},
  server::ButtplugServerResultFuture,
};
use async_trait::async_trait;
use futures::future;
use std::collections::HashMap;

macro_rules! create_protocols(
  (
    $(
      ($protocol_config_name:tt, $protocol_module:tt, $protocol_name:tt)
    ),*
  ) => {
    paste::item! {
      $(
        use $protocol_module::[<$protocol_name Creator>];
      )*

      pub fn create_protocol_creator_map() -> HashMap::<String, ProtocolConstructor> {
        // Do not try to use HashMap::new() here. We need the explicit typing,
        // otherwise we'll just get an anonymous closure type during insert that
        // won't match.
        let mut protocols = HashMap::<String, ProtocolConstructor>::new();

        $(
          protocols.insert(
            $protocol_config_name.to_owned(),
            Box::new(|config: DeviceProtocolConfiguration| {
              Box::new([<$protocol_name Creator>]::new(config))
            }),
          );
        )*
        protocols
      }
    }
  }
);

// IF YOU WANT TO ADD NEW PROTOCOLS TO THE SYSTEM, DO IT HERE.
//
// You'll also need to add a mod call up top, because having mod calls in the
// macro does weird things to IDEs.
//
// This takes a tuple per protocol:
//
// - the name of the protocol in the device configuration file
// - the name of the module
// - the base name of the protocol, as used in create_buttplug_protocol!

create_protocols!(
  ("aneros", aneros, Aneros),
  ("maxpro", maxpro, Maxpro),
  ("lovense", lovense, Lovense),
  ("picobong", picobong, Picobong),
  ("realov", realov, Realov),
  ("prettylove", prettylove, PrettyLove),
  ("svakom", svakom, Svakom),
  ("youcups", youcups, Youcups),
  ("youou", youou, Youou),
  ("lovehoney-desire", lovehoney_desire, LovehoneyDesire),
  ("vorze-sa", vorze_sa, VorzeSA),
  ("xinput", xinput, XInput)
);

#[async_trait]
pub trait ButtplugProtocolCreator: Sync + Send {
  async fn try_create_protocol(
    &self,
    device_impl: &dyn DeviceImpl,
  ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError>;
}

// These properties are divided up so we can spread between macro generated and
// hand written implementations.
pub trait ButtplugProtocol: Send + ButtplugProtocolCommandHandler {}

pub trait ButtplugProtocolProperties {
  fn name(&self) -> &str;
  fn message_attributes(&self) -> MessageAttributesMap;
  fn stop_commands(&self) -> Vec<ButtplugDeviceCommandMessageUnion>;
}

pub trait ButtplugProtocolCommandHandler: Send + ButtplugProtocolProperties {
  fn handle_command(
    &self,
    device: &dyn DeviceImpl,
    command_message: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugServerResultFuture {
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
      ButtplugDeviceCommandMessageUnion::SubscribeCmd(msg) => {
        self.handle_subscribe_cmd(device, msg)
      }
      ButtplugDeviceCommandMessageUnion::UnsubscribeCmd(msg) => {
        self.handle_unsubscribe_cmd(device, msg)
      }
      ButtplugDeviceCommandMessageUnion::VibrateCmd(msg) => self.handle_vibrate_cmd(device, msg),
      ButtplugDeviceCommandMessageUnion::VorzeA10CycloneCmd(msg) => {
        self.handle_vorze_a10_cyclone_cmd(device, msg)
      }
    }
  }

  fn handle_stop_device_cmd(
    &self,
    device: &dyn DeviceImpl,
    message: messages::StopDeviceCmd,
  ) -> ButtplugServerResultFuture {
    let ok_return = messages::Ok::new(message.get_id());
    let fut_vec: Vec<ButtplugServerResultFuture> = self.stop_commands().iter().map(|cmd| self.handle_command(device, cmd.clone())).collect();
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
    device: &dyn DeviceImpl,
    message: messages::SingleMotorVibrateCmd,
  ) -> ButtplugServerResultFuture {
        // Time for sadness! In order to handle conversion of
        // SingleMotorVibrateCmd, we need to know how many
        // vibrators a device has. We don't actually know that
        // until we get to the protocol level, so we're stuck
        // parsing this here. Since we can assume
        // SingleMotorVibrateCmd will ALWAYS map to vibration,
        // we can convert to VibrateCmd here and save ourselves
        // having to handle it in every protocol, meaning spec
        // v0 and v1 programs will still be forward compatible
        // with vibrators.
        let vibrator_count;
        if let Some(attr) = self.message_attributes().get(&ButtplugDeviceMessageType::VibrateCmd) {
          if let Some(count) = attr.feature_count {
            vibrator_count = count as usize;
          } else {
            return ButtplugDeviceError::new("$protocol_name needs to support VibrateCmd with a feature count to use SingleMotorVibrateCmd.").into();
          }
        } else {
          return ButtplugDeviceError::new("$protocol_name needs to support VibrateCmd to use SingleMotorVibrateCmd.").into();
        }
        let speed = message.speed;
        let mut cmds = vec!();
        for i in 0..vibrator_count {
          cmds.push(VibrateSubcommand::new(i as u32, speed));
        }
        let mut vibrate_cmd = VibrateCmd::new(message.device_index, cmds);
        vibrate_cmd.set_id(message.get_id());
        self.handle_command(device, vibrate_cmd.into())
  }

  fn handle_raw_write_cmd(
    &self,
    _device: &dyn DeviceImpl,
    _message: messages::RawWriteCmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented()
  }

  fn handle_raw_read_cmd(
    &self,
    _device: &dyn DeviceImpl,
    _message: messages::RawReadCmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented()
  }

  fn command_unimplemented(&self) -> ButtplugServerResultFuture {
    #[cfg(build = "debug")]
    unimplemented!("Command not implemented for this protocol");
    #[cfg(not(build = "debug"))]
    Box::pin(future::ready(Err(
      ButtplugDeviceError::new("Command not implemented for this protocol").into(),
    )))
  }

  fn handle_vorze_a10_cyclone_cmd(
    &self,
    _device: &dyn DeviceImpl,
    _message: messages::VorzeA10CycloneCmd
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented()
  }

  fn handle_unsubscribe_cmd(
    &self,
    _device: &dyn DeviceImpl,
    _message: messages::UnsubscribeCmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented()
  }

  fn handle_subscribe_cmd(
    &self,
    _device: &dyn DeviceImpl,
    _message: messages::SubscribeCmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented()
  }

  fn handle_kiiroo_cmd(
    &self,
    _device: &dyn DeviceImpl,
    _message: messages::KiirooCmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented()
  }

  fn handle_fleshlight_launch_fw12_cmd(
    &self,
    _device: &dyn DeviceImpl,
    _message: messages::FleshlightLaunchFW12Cmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented()
  }

  fn handle_vibrate_cmd(
    &self,
    _device: &dyn DeviceImpl,
    _message: messages::VibrateCmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented()
  }

  fn handle_rotate_cmd(
    &self,
    _device: &dyn DeviceImpl,
    _message: messages::RotateCmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented()
  }

  fn handle_linear_cmd(
    &self,
    _device: &dyn DeviceImpl,
    _message: messages::LinearCmd,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented()
  }
/*
  fn handle_battery_level_cmd(
    &self,
    device: &dyn DeviceImpl,
    message: messages::Battery,
  ) -> ButtplugServerResultFuture {
    self.command_unimplemented()
  }

  fn handle_rssi_level_cmd(
    &self,
    device: &dyn DeviceImpl,
    message: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugServerResultFuture {
    unimplemented!("Command not implemented for this protocol");
  }
  */
}

// TODO These macros could use some compilation tests to make sure we're
// bringing in everything we need.

// Note: We have to use tt instead of ident here due to the async_trait macro.
// See https://github.com/dtolnay/async-trait/issues/46 for more info.
#[macro_export]
macro_rules! create_protocol_creator_impl (
  (
    true,
    $protocol_name:tt
  ) => {
    use async_trait::async_trait;
    use crate::{
      device::{
        protocol::{ButtplugProtocol, ButtplugProtocolCreator},
        configuration_manager::DeviceProtocolConfiguration,
      },
    };

    paste::item! {
      pub struct [<$protocol_name Creator>] {
        config: DeviceProtocolConfiguration,
      }

      impl [<$protocol_name Creator>] {
        pub fn new(config: DeviceProtocolConfiguration) -> Self {
          Self { config }
        }
      }

      #[async_trait]
      impl ButtplugProtocolCreator for [<$protocol_name Creator>] {
        async fn try_create_protocol(
          &self,
          device_impl: &dyn DeviceImpl,
        ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError> {
          let (names, attrs) = self.config.get_attributes(device_impl.name()).unwrap();
          let name = names.get("en-us").unwrap();
          Ok(Box::new($protocol_name::new(name, attrs)))
        }
      }
    }
  };
  (
    false,
    $protocol_name:tt
  ) => {
  };
);

#[macro_export]
macro_rules! create_buttplug_protocol (
  (
    $protocol_name:tt,
    $create_protocol_creator_impl:tt,
    (
      $(
        ( $member_name:tt: $member_type:ty = $member_initial_value:expr )
      ),*
    )
  ) => {
    use crate::{
      create_protocol_creator_impl,
      device::{
        Endpoint, DeviceWriteCmd, DeviceImpl,
        protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
      },
      core::{
        errors::{ButtplugError},
        messages::{
          self,
          MessageAttributesMap,
          ButtplugDeviceCommandMessageUnion,
        },
      },
      server::ButtplugServerResultFuture
    };
    use std::cell::RefCell;
    #[allow(unused_imports)]
    use futures::future;

    create_protocol_creator_impl!($create_protocol_creator_impl, $protocol_name);

    pub struct $protocol_name {
      name: String,
      attributes: MessageAttributesMap,
      #[allow(dead_code)]
      manager: RefCell<GenericCommandManager>,
      stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
      $(
        $member_name: $member_type
      ),*
    }

    paste::item! {
      impl $protocol_name {
        pub fn new(name: &str, attributes: MessageAttributesMap) -> Self {
          let manager = GenericCommandManager::new(&attributes);

          $protocol_name {
            name: name.to_owned(),
            attributes,
            stop_commands: manager.get_stop_commands(),
            manager: RefCell::new(manager),
            $(
              $member_name: $member_initial_value
            ),*
          }
        }
      }

      impl ButtplugProtocol for $protocol_name {}

      impl ButtplugProtocolProperties for $protocol_name {
        fn name(&self) -> &str {
          &self.name
        }

        fn message_attributes(&self) -> MessageAttributesMap {
          self.attributes.clone()
        }

        fn stop_commands(&self) -> Vec<ButtplugDeviceCommandMessageUnion> {
          self.stop_commands.clone()
        }
      }
    }
  }
);
