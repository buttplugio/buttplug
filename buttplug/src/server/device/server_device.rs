// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.



use std::{
  fmt::{self, Debug},
  sync::Arc,
};

use crate::{
  core::{
    errors::{ButtplugError, ButtplugDeviceError},
    messages::{
      self,
      ButtplugMessage,
      ButtplugDeviceMessage,
      ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceMessageType,
      ButtplugServerDeviceMessage,
      DeviceMessageAttributesMap,
      Endpoint,
      RawReading,
      RawSubscribeCmd,
      VibrateCmd,
      VibrateSubcommand,
    },
    ButtplugResultFuture,
  },
  server::{
    ButtplugServerResultFuture,
    device::{
      hardware::{Hardware, HardwareConnector, HardwareEvent, HardwareCommand},
      configuration::{ProtocolAttributesType, DeviceConfigurationManager},
      protocol::{ProtocolHandler}
    },
  }, util::stream::convert_broadcast_receiver_to_stream,
};
use getset::{Getters, Setters, MutGetters};
use serde::{Serialize, Deserialize};
use core::hash::{Hash, Hasher};
use futures::{future, StreamExt};

use super::{configuration::ProtocolDeviceAttributes, protocol::generic_command_manager::GenericCommandManager};

#[derive(Debug)]
pub enum ServerDeviceEvent {
  Connected(Arc<ServerDevice>),
  Notification(ServerDeviceIdentifier, ButtplugServerDeviceMessage),
  Disconnected(ServerDeviceIdentifier)
}

/// Identifying information for a connected devices
/// 
/// Contains the 3 fields needed to uniquely identify a device in the system.
#[derive(Debug, Eq, PartialEq, Hash, Clone, Getters, Setters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub(crate)", get_mut = "pub(crate)")]
pub struct ServerDeviceIdentifier {
  /// Address, as possibly serialized by whatever the managing library for the Device Communication Manager is.
  address: String,
  /// Name of the protocol used
  protocol: String,
  /// Internal identifier for the protocol used
  attributes_identifier: ProtocolAttributesType,
}

impl ServerDeviceIdentifier {
  /// Creates a new instance
  pub fn new(address: &str, protocol: &str, identifier: &ProtocolAttributesType) -> Self {
    Self {
      address: address.to_owned(),
      protocol: protocol.to_owned(),
      attributes_identifier: identifier.clone()
    }
  }
}

pub(super) async fn build_server_device(device_config_manager: Arc<DeviceConfigurationManager>, mut hardware_connector: Box<dyn HardwareConnector>) -> Result<Option<ServerDevice>, ButtplugDeviceError> {

  // First off, we need to see if we even have a configuration available for the device we're
  // trying to create. If we don't, exit, because this isn't actually an error. However, if we
  // *do* have a configuration but something goes wrong after this, then it's an error.
  let protocol_specializers = device_config_manager.protocol_specializers(&hardware_connector.specifier()); 

  // If we have no identifiers, then there's nothing to do here. Throw an error.
  if protocol_specializers.is_empty() {
    debug!("{}", format!("No viable protocols for hardware {:?}, ignoring.", hardware_connector.specifier()));
    return Ok(None)    
  }

  // At this point, we know we've got hardware that is waiting to connect, and enough protocol
  // info to actually do something after we connect. So go ahead and connect.
  let mut hardware_specializer = hardware_connector.connect().await?;

  // We can't run these in parallel because we need to only accept one specializer.
  let mut protocol_identifier = None;
  let mut hardware_out = None;
  for protocol_specializer in protocol_specializers {
    if let Ok(specialized_hardware) = hardware_specializer.specialize(protocol_specializer.specifiers()).await {
      protocol_identifier = Some(protocol_specializer.identify());
      hardware_out = Some(specialized_hardware);
      break;
    }
  }

  if protocol_identifier.is_none() {
    return Err(ButtplugDeviceError::DeviceConfigurationError("No protocols with viable communication matches for hardware.".to_owned()));
  }

  let mut protocol_identifier_stage = protocol_identifier.unwrap();
  let hardware = Arc::new(hardware_out.unwrap());

  let (identifier, mut protocol_initializer) = protocol_identifier_stage.identify(hardware.clone()).await?;

  // Now we have an identifier. After this point, if anything fails, consider it a complete
  // connection failure, as identify may have already run commands on the device, and therefore
  // put it in an unknown state if anything fails.
  
  // Check in the DeviceConfigurationManager to make sure we have attributes
  // for this device.
  let attrs = if let Some(attrs) = device_config_manager.protocol_device_attributes(&identifier, &hardware.endpoints()) {
    attrs
  } else {
    return Err(ButtplugDeviceError::DeviceConfigurationError(format!("No protocols with viable protocol attributes for hardware {:?}.", identifier)));
  };

  // If we have attributes, go ahead and initialize, handing us back our hardware instance that
  // is now ready to use with the protocol handler.
  
  // Build the server device and return.
  
  let handler = protocol_initializer.initialize(hardware.clone()).await?;

  // We now have fully initialized hardware, return a server device.
  Ok(Some(ServerDevice::new(
    identifier,
    handler,
    hardware,
    &attrs
  )))
}

pub struct ServerDevice {
  hardware: Arc<Hardware>,
  handler: Box<dyn ProtocolHandler>,
  attributes: ProtocolDeviceAttributes,
  generic_command_manager: GenericCommandManager,
  /// Unique identifier for the device
  identifier: ServerDeviceIdentifier

}
impl Debug for ServerDevice {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ButtplugDevice")
      .field("name", &self.name())
      .field("identifier", &self.identifier)
      .finish()
  }
}

impl Hash for ServerDevice {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.identifier.hash(state);
  }
}

impl Eq for ServerDevice {
}

impl PartialEq for ServerDevice {
  fn eq(&self, other: &Self) -> bool {
    self.identifier == *other.identifier()
  }
}

impl ServerDevice {
  /// Given a protocol and a device impl, create a new ButtplugDevice instance
  fn new(identifier: ServerDeviceIdentifier, handler: Box<dyn ProtocolHandler>, hardware: Arc<Hardware>, attributes: &ProtocolDeviceAttributes) -> Self {

    // Hook up our stream mapper now.

    Self {
      identifier,
      generic_command_manager: GenericCommandManager::new(attributes),
      handler,
      hardware,
      attributes: attributes.clone()
    }
  }

  /// Returns the device identifier
  pub fn identifier(&self) -> &ServerDeviceIdentifier {
    &self.identifier
  }

  /// Get the user created display name for a device, if one exists.
  pub fn display_name(&self) -> Option<String> {
    self.attributes.display_name()
  }

  /// Get the name of the device as set in the Device Configuration File.
  /// 
  /// This will also append "(Raw Messaged Allowed)" to the device name if raw mode is on, to warn
  /// users that the device is capable of direct communication.
  pub fn name(&self) -> String {
    // Instead of checking for raw messages at the protocol level, add the raw
    // call here, since this is the only way to access devices in the library
    // anyways.
    //
    // Having raw turned on means it'll work for read/write/sub/unsub on any
    // endpoint so just use an arbitrary message here to check.
    if self
      .supports_message(&ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(
        RawSubscribeCmd::new(1, Endpoint::Tx),
      ))
      .is_ok()
    {
      format!("{} (Raw Messages Allowed)", self.attributes.name())
    } else {
      self.attributes.name().to_owned()
    }
  }

  /// Disconnect from the device, if it's connected.
  pub fn disconnect(&self) -> ButtplugResultFuture {
    let fut = self.hardware.disconnect();
    Box::pin(async move { 
      fut.await.map_err(|err| err.into())
    })
  }

  /// Retreive the message attributes for the device.
  pub fn message_attributes(&self) -> DeviceMessageAttributesMap {
    self.attributes.message_attributes_map()
  }

  /// Retreive the event stream for the device.
  /// 
  /// This will include connections, disconnections, and notification events from subscribed
  /// endpoints.
  pub fn event_stream(&self) -> impl futures::Stream<Item = ServerDeviceEvent> {
    let identifier = self.identifier.clone();
    convert_broadcast_receiver_to_stream(self.hardware.event_stream()).map(move |hardware_event| {
      let id = identifier.clone();
      match hardware_event {
        HardwareEvent::Disconnected(_) => ServerDeviceEvent::Disconnected(id),
        HardwareEvent::Notification(_address, endpoint,  data) => {
          // TODO Figure out how we're going to parse raw data into something sendable to the client.
          ServerDeviceEvent::Notification(id, ButtplugServerDeviceMessage::RawReading(RawReading::new(0, endpoint, data)))
        }
      }
    })
  }  
  
  pub fn supports_message(
    &self,
    message: &ButtplugDeviceCommandMessageUnion,
  ) -> Result<(), ButtplugError> {
    // TODO This should be generated by a macro, as should the types enum.
    let check_msg = |msg_type| self
    .attributes
    .allows_message(&msg_type)
    .then(|| ())
    .ok_or(ButtplugDeviceError::MessageNotSupported(msg_type));

    match message {
      ButtplugDeviceCommandMessageUnion::BatteryLevelCmd(_) => check_msg(ButtplugDeviceMessageType::BatteryLevelCmd),
      ButtplugDeviceCommandMessageUnion::FleshlightLaunchFW12Cmd(_) => check_msg(ButtplugDeviceMessageType::FleshlightLaunchFW12Cmd),
      ButtplugDeviceCommandMessageUnion::KiirooCmd(_) => check_msg(ButtplugDeviceMessageType::KiirooCmd),
      ButtplugDeviceCommandMessageUnion::LinearCmd(_) => check_msg(ButtplugDeviceMessageType::LinearCmd),
      ButtplugDeviceCommandMessageUnion::RawReadCmd(_) => check_msg(ButtplugDeviceMessageType::RawReadCmd),
      ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(_) => check_msg(ButtplugDeviceMessageType::RawSubscribeCmd),
      ButtplugDeviceCommandMessageUnion::RawUnsubscribeCmd(_) => check_msg(ButtplugDeviceMessageType::RawUnsubscribeCmd),
      ButtplugDeviceCommandMessageUnion::RawWriteCmd(_) => check_msg(ButtplugDeviceMessageType::RawWriteCmd),
      ButtplugDeviceCommandMessageUnion::RotateCmd(_) => check_msg(ButtplugDeviceMessageType::RotateCmd),
      ButtplugDeviceCommandMessageUnion::RSSILevelCmd(_) => check_msg(ButtplugDeviceMessageType::RSSILevelCmd),
      ButtplugDeviceCommandMessageUnion::LevelCmd(_) => check_msg(ButtplugDeviceMessageType::LevelCmd),
      // We translate SingleMotorVibrateCmd into Vibrate, so this one is special.
      ButtplugDeviceCommandMessageUnion::SingleMotorVibrateCmd(_) => check_msg(ButtplugDeviceMessageType::VibrateCmd),
      ButtplugDeviceCommandMessageUnion::StopDeviceCmd(_) => check_msg(ButtplugDeviceMessageType::StopDeviceCmd),
      ButtplugDeviceCommandMessageUnion::VibrateCmd(_) => check_msg(ButtplugDeviceMessageType::VibrateCmd),
      ButtplugDeviceCommandMessageUnion::VorzeA10CycloneCmd(_) => check_msg(ButtplugDeviceMessageType::VorzeA10CycloneCmd),
    }.map_err(|err| err.into())
  }
  
  // In order to not have to worry about id setting at the protocol level (this
  // should be taken care of in the server's device manager), we return server
  // messages but Buttplug errors.
  pub fn parse_message(
    &self,
    command_message: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugServerResultFuture {
    if let Err(err) = self.supports_message(&command_message) {
      return Box::pin(future::ready(Err(err)));
    }

    // If a handler implements handle message, bypass all of our parsing and let it do its own
    // thing. This should be a very rare thing.
    if self.handler.has_handle_message() {
      let fut = self.handle_generic_command_result(self.handler.handle_message(&command_message));
      return Box::pin(async move {
        fut.await
      });
    }

    match command_message {
      // messages we can handle in this struct
      ButtplugDeviceCommandMessageUnion::RawReadCmd(msg) => self.handle_raw_read_cmd(msg),
      ButtplugDeviceCommandMessageUnion::RawWriteCmd(msg) => self.handle_raw_write_cmd(msg),
      ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(msg) => self.handle_raw_subscribe_cmd(msg),
      ButtplugDeviceCommandMessageUnion::RawUnsubscribeCmd(msg) => self.handle_raw_unsubscribe_cmd(msg),
      ButtplugDeviceCommandMessageUnion::StopDeviceCmd(_) => self.handle_stop_device_cmd(),
      ButtplugDeviceCommandMessageUnion::SingleMotorVibrateCmd(msg) => self.handle_single_motor_vibrate_cmd(msg),
      ButtplugDeviceCommandMessageUnion::BatteryLevelCmd(_) => Box::pin(future::ready(Err(ButtplugDeviceError::ProtocolNotImplemented("Being Lazy".to_owned()).into()))), // self.handle_battery_level_cmd(msg),
      ButtplugDeviceCommandMessageUnion::RSSILevelCmd(_) => Box::pin(future::ready(Err(ButtplugDeviceError::ProtocolNotImplemented("Being Lazy".to_owned()).into()))), //self.handle_rssi_level_cmd(msg),

      // Message that return lists of hardware commands which we'll handle sending to the devices
      // here, in order to reduce boilerplate in the implementations. Generic messages that we can
      // use the generic command manager for, but still need protocol level translation.
      ButtplugDeviceCommandMessageUnion::VibrateCmd(msg) => {
        let commands = match self.generic_command_manager.update_vibration(&msg, self.handler.needs_full_command_set()) {
          Ok(values) => values,
          Err(err) => return Box::pin(future::ready(Err(err)))
        };
        self.handle_generic_command_result(self.handler.handle_vibrate_cmd(&commands))
      }
      ButtplugDeviceCommandMessageUnion::RotateCmd(msg) => {
        let commands = match self.generic_command_manager.update_rotation(&msg) {
          Ok(values) => values,
          Err(err) => return Box::pin(future::ready(Err(err)))
        };
        self.handle_generic_command_result(self.handler.handle_rotate_cmd(&commands))
      }
      ButtplugDeviceCommandMessageUnion::LevelCmd(msg) => self.handle_generic_command_result(self.handler.handle_level_cmd(msg)),
      ButtplugDeviceCommandMessageUnion::LinearCmd(msg) => self.handle_generic_command_result(self.handler.handle_linear_cmd(msg)),
      ButtplugDeviceCommandMessageUnion::FleshlightLaunchFW12Cmd(msg) => self.handle_generic_command_result(self.handler.handle_fleshlight_launch_fw12_cmd(msg)),
      ButtplugDeviceCommandMessageUnion::VorzeA10CycloneCmd(msg) => self.handle_generic_command_result(self.handler.handle_vorze_a10_cyclone_cmd(msg)),

      // Everything else, which is mostly older messages, or special things that require reads.
      ButtplugDeviceCommandMessageUnion::KiirooCmd(_) => Box::pin(future::ready(Err(ButtplugDeviceError::ProtocolNotImplemented("Being Lazy".to_owned()).into()))) //self.handler.handle_kiiroo_cmd( msg),
    }
  }

  fn handle_hardware_commands(
    &self,
    commands: Vec<HardwareCommand>
  ) -> ButtplugServerResultFuture {
    let hardware = self.hardware.clone();
    Box::pin(async move {
      // Run commands in order, otherwise we may end up sending out of order. This may take a while,
      // but it's what 99% of protocols expect. If they want something else, they can implement it
      // themselves.
      //
      // If anything errors out, just bail on the command series. This most likely means the device
      // disconnected.
      for command in commands {
        hardware.parse_message(&command).await?;
      }
      Ok(messages::Ok::default().into())
    })
  }

  fn handle_generic_command_result(
    &self,
    command_result: Result<Vec<HardwareCommand>, ButtplugDeviceError>
  ) -> ButtplugServerResultFuture {
    let hardware_commands = match command_result {
      Ok(commands) => commands,
      Err(err) => return Box::pin(future::ready(Err(err.into())))
    };
    
    self.handle_hardware_commands(hardware_commands)
  }

  fn handle_stop_device_cmd(
    &self,
  ) -> ButtplugServerResultFuture {
    let commands = self.generic_command_manager.stop_commands();
    let mut fut_vec = vec![];
    commands
      .iter()
      .for_each(|msg| fut_vec.push(self.parse_message(msg.clone())));
    Box::pin(async move {
      for fut in fut_vec {
        fut.await?;
      }
      Ok(messages::Ok::default().into())
    })
  }

  fn handle_single_motor_vibrate_cmd(
    &self,
    message: messages::SingleMotorVibrateCmd,
  ) -> ButtplugServerResultFuture {
    let vibrator_count;
    if let Some(attr) = self
      .attributes
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
    self.parse_message(vibrate_cmd.into())
  }

  fn handle_raw_write_cmd(
    &self,
    message: messages::RawWriteCmd,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = self.hardware.write_value(&message.into());
    Box::pin(async move { fut.await.map(|_| messages::Ok::new(id).into()).map_err(|err| err.into()) })
  }

  fn handle_raw_read_cmd(
    &self,
    message: messages::RawReadCmd,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = self.hardware.read_value(&message.into());
    Box::pin(async move {
      fut.await.map(|mut msg| {
        msg.set_id(id);
        msg.into()
      })
      .map_err(|err| err.into())
    })
  }

  fn handle_raw_unsubscribe_cmd(
    &self,
    message: messages::RawUnsubscribeCmd,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = self.hardware.unsubscribe(&message.into());
    Box::pin(async move { fut.await.map(|_| messages::Ok::new(id).into()).map_err(|err| err.into()) })
  }

  fn handle_raw_subscribe_cmd(
    &self,
    message: messages::RawSubscribeCmd,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = self.hardware.subscribe(&message.into());
    Box::pin(async move { fut.await.map(|_| messages::Ok::new(id).into()).map_err(|err| err.into()) })
  }
}
