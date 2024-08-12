// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Server Device Implementation
//!
//! This struct manages the trip from buttplug protocol actuator/sensor/raw message to hardware
//! communication. This involves:
//!
//! - Taking buttplug device command messages from the exposed server
//! - Converting older spec version messages to the newest spec version, which usually requires
//!   device information for actuation/sensor messages.
//! - Validity checking the messages to make sure they match the capabilities of the hardware
//! - Turning the buttplug messages into hardware commands via the associated protocol
//! - Sending them to the hardware
//! - Possibly receiving back information (in the case of sensors), possibly firing and forgetting
//!   (in terms of almost everything else)
//!
//! We make a lot of assumptions in here based on the devices we support right now, including:
//!
//! - Devices will only ever have one directional rotation actuator (we have no device that supports
//!   two rotational components currently)
//! - Devices will only ever have one linear actuator (we have no device that supports multiple
//!   linear actuators currently)
//! - Devices scalar command ordering is explicitly set by the device config file
//!   - This means that we rely on the config file to know which vibrator is which on a device with
//!     multiple vibrators. In protocols, especially for toy brands that release a large line of
//!     different toys all using the same protocols (lovense, wevibe, etc), the order of features in
//!     the config file MATTERS and needs to be tested against an actual device to make sure we're
//!     controlling the actuator we think we are.
//!   - This situation sucks and we should have better definitions, a problem outlined at
//!     https://github.com/buttplugio/buttplug/issues/646
//!
//! In order to handle multiple message spec versions

use std::{
  fmt::{self, Debug},
  sync::Arc,
  time::Duration,
};

use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    message::{
      self,
      ActuatorType,
      BatteryLevelReading,
      ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceMessage,
      ButtplugDeviceMessageType,
      ButtplugMessage,
      ButtplugServerDeviceMessage,
      Endpoint,
      FeatureType,
      RSSILevelReading,
      RawReading,
      RawSubscribeCmd,
      ScalarCmd,
      ScalarCmdV4,
      ScalarSubcommandV4,
      SensorReadCmd,
      SensorReadCmdV4,
      SensorReading,
      SensorReadingV4,
      SensorSubscribeCmd,
      SensorSubscribeCmdV4,
      SensorType,
      SensorUnsubscribeCmdV4,
      VibrateCmd,
    },
    ButtplugResultFuture,
  },
  server::{
    device::{
      configuration::DeviceConfigurationManager,
      hardware::{Hardware, HardwareCommand, HardwareConnector, HardwareEvent},
      protocol::ProtocolHandler,
    },
    ButtplugServerResultFuture,
  },
  util::{self, async_manager, stream::convert_broadcast_receiver_to_stream},
};
use core::hash::{Hash, Hasher};
use dashmap::DashSet;
use futures::future::{self, BoxFuture, FutureExt};
use getset::Getters;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;

use super::{
  configuration::{UserDeviceDefinition, UserDeviceIdentifier},
  hardware::HardwareWriteCmd,
  protocol::{
    actuator_command_manager::ActuatorCommandManager,
    ProtocolKeepaliveStrategy,
    ProtocolSpecializer,
  },
};

#[derive(Debug)]
pub enum ServerDeviceEvent {
  Connected(Arc<ServerDevice>),
  Notification(UserDeviceIdentifier, ButtplugServerDeviceMessage),
  Disconnected(UserDeviceIdentifier),
}

#[derive(Getters)]
pub struct ServerDevice {
  hardware: Arc<Hardware>,
  handler: Arc<dyn ProtocolHandler>,
  #[getset(get = "pub")]
  definition: UserDeviceDefinition,
  actuator_command_manager: ActuatorCommandManager,
  /// Unique identifier for the device
  #[getset(get = "pub")]
  identifier: UserDeviceIdentifier,
  raw_subscribed_endpoints: Arc<DashSet<Endpoint>>,
  keepalive_packet: Arc<RwLock<Option<HardwareWriteCmd>>>,
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
  pub(super) async fn build(
    device_config_manager: Arc<DeviceConfigurationManager>,
    mut hardware_connector: Box<dyn HardwareConnector>,
    protocol_specializers: Vec<ProtocolSpecializer>,
  ) -> Result<Self, ButtplugDeviceError> {
    // We've already checked to make sure we have specializers in the server device manager event
    // loop. That check used to be here for sake of continuity in building devices in this method, but
    // having that done before we get here fixes issues with some device advertisement timing (See
    // #462 for more info.)

    // At this point, we know we've got hardware that is waiting to connect, and enough protocol
    // info to actually do something after we connect. So go ahead and connect.
    trace!("Connecting to {:?}", hardware_connector);
    let mut hardware_specializer = hardware_connector.connect().await?;

    // We can't run these in parallel because we need to only accept one specializer.
    let mut protocol_identifier = None;
    let mut hardware_out = None;
    for protocol_specializer in protocol_specializers {
      if let Ok(specialized_hardware) = hardware_specializer
        .specialize(protocol_specializer.specifiers())
        .await
      {
        protocol_identifier = Some(protocol_specializer.identify());
        hardware_out = Some(specialized_hardware);
        break;
      }
    }

    if protocol_identifier.is_none() {
      return Err(ButtplugDeviceError::DeviceConfigurationError(
        "No protocols with viable communication matches for hardware.".to_owned(),
      ));
    }

    let mut protocol_identifier_stage = protocol_identifier.unwrap();
    let hardware = Arc::new(hardware_out.unwrap());

    let (identifier, mut protocol_initializer) = protocol_identifier_stage
      .identify(hardware.clone(), hardware_connector.specifier())
      .await?;

    // Now we have an identifier. After this point, if anything fails, consider it a complete
    // connection failure, as identify may have already run commands on the device, and therefore
    // put it in an unknown state if anything fails.

    // Check in the DeviceConfigurationManager to make sure we have attributes for this device.
    let attrs = if let Some(attrs) =
      device_config_manager.device_definition(&identifier, &hardware.endpoints())
    {
      attrs
    } else {
      return Err(ButtplugDeviceError::DeviceConfigurationError(format!(
        "No protocols with viable protocol attributes for hardware {:?}.",
        identifier
      )));
    };

    // If we have attributes, go ahead and initialize, handing us back our hardware instance that
    // is now ready to use with the protocol handler.

    // Build the server device and return.
    let handler = protocol_initializer
      .initialize(hardware.clone(), &attrs.clone().into())
      .await?;

    let requires_keepalive = hardware.requires_keepalive();
    let strategy = handler.keepalive_strategy();

    // We now have fully initialized hardware, return a server device.
    let device = Self::new(identifier, handler, hardware, &attrs);

    // If we need a keepalive with a packet replay, set this up via stopping the device on connect.
    if requires_keepalive
      && matches!(
        strategy,
        ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
      )
    {
      if let Err(e) = device.handle_stop_device_cmd().await {
        return Err(ButtplugDeviceError::DeviceConnectionError(format!(
          "Error setting up keepalive: {}",
          e
        )));
      }
    }

    Ok(device)
  }

  /// Given a protocol and a device impl, create a new ButtplugDevice instance
  fn new(
    identifier: UserDeviceIdentifier,
    handler: Arc<dyn ProtocolHandler>,
    hardware: Arc<Hardware>,
    definition: &UserDeviceDefinition,
  ) -> Self {
    let keepalive_packet = Arc::new(RwLock::new(None));
    let acm = ActuatorCommandManager::new(definition.features());
    // If we've gotten here, we know our hardware is connected. This means we can start the keepalive if it's required.
    if hardware.requires_keepalive()
      && !matches!(
        handler.keepalive_strategy(),
        ProtocolKeepaliveStrategy::NoStrategy
      )
    {
      let hardware = hardware.clone();
      let strategy = handler.keepalive_strategy();
      let keepalive_packet = keepalive_packet.clone();
      async_manager::spawn(async move {
        // Arbitrary wait time for now.
        let wait_duration = Duration::from_secs(5);
        loop {
          if hardware.time_since_last_write().await > wait_duration {
            match &strategy {
              ProtocolKeepaliveStrategy::RepeatPacketStrategy(packet) => {
                if let Err(e) = hardware.write_value(&packet).await {
                  warn!("Error writing keepalive packet: {:?}", e);
                  break;
                }
              }
              ProtocolKeepaliveStrategy::RepeatLastPacketStrategy => {
                if let Some(packet) = &*keepalive_packet.read().await {
                  if let Err(e) = hardware.write_value(&packet).await {
                    warn!("Error writing keepalive packet: {:?}", e);
                    break;
                  }
                }
              }
              _ => {
                info!(
                  "Protocol keepalive strategy {:?} not implemented, replacing with NoStrategy",
                  strategy
                );
              }
            }
          }
          // Arbitrary wait time for now.
          util::sleep(wait_duration).await;
        }
        info!("Leaving keepalive task for {}", hardware.name());
      });
    }

    Self {
      identifier,
      actuator_command_manager: acm,
      handler,
      hardware,
      keepalive_packet,
      definition: definition.clone(),
      raw_subscribed_endpoints: Arc::new(DashSet::new()),
    }
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
      format!("{} (Raw Messages Allowed)", self.definition.name())
    } else {
      self.definition.name().to_owned()
    }
  }

  /// Disconnect from the device, if it's connected.
  pub fn disconnect(&self) -> ButtplugResultFuture {
    let fut = self.hardware.disconnect();
    async move { fut.await.map_err(|err| err.into()) }.boxed()
  }

  /// Retreive the event stream for the device.
  ///
  /// This will include connections, disconnections, and notification events from subscribed
  /// endpoints.
  pub fn event_stream(&self) -> impl futures::Stream<Item = ServerDeviceEvent> + Send {
    let identifier = self.identifier.clone();
    let raw_endpoints = self.raw_subscribed_endpoints.clone();
    let hardware_stream = convert_broadcast_receiver_to_stream(self.hardware.event_stream())
      .filter_map(move |hardware_event| {
        let id = identifier.clone();
        match hardware_event {
          HardwareEvent::Disconnected(_) => Some(ServerDeviceEvent::Disconnected(id)),
          HardwareEvent::Notification(_address, endpoint, data) => {
            // TODO Figure out how we're going to parse raw data into something sendable to the client.
            if raw_endpoints.contains(&endpoint) {
              Some(ServerDeviceEvent::Notification(
                id,
                ButtplugServerDeviceMessage::RawReading(RawReading::new(0, endpoint, data)),
              ))
            } else {
              None
            }
          }
        }
      });

    let identifier = self.identifier.clone();
    let handler_mapped_stream = self.handler.event_stream().map(move |incoming_message| {
      let id = identifier.clone();
      ServerDeviceEvent::Notification(id, incoming_message)
    });
    hardware_stream.merge(handler_mapped_stream)
  }

  pub fn supports_message(
    &self,
    message: &ButtplugDeviceCommandMessageUnion,
  ) -> Result<(), ButtplugError> {
    // TODO This should be generated by a macro, as should the types enum.
    let check_msg = |msg_type| {
      self
        .definition
        .allows_message(&msg_type)
        .then_some(())
        .ok_or(ButtplugDeviceError::MessageNotSupported(msg_type))
    };

    match message {
      ButtplugDeviceCommandMessageUnion::BatteryLevelCmd(_) => {
        //check_msg(ButtplugDeviceMessageType::BatteryLevelCmd)
        check_msg(ButtplugDeviceMessageType::SensorReadCmd)
      }
      ButtplugDeviceCommandMessageUnion::FleshlightLaunchFW12Cmd(_) => {
        check_msg(ButtplugDeviceMessageType::FleshlightLaunchFW12Cmd)
      }
      ButtplugDeviceCommandMessageUnion::KiirooCmd(_) => {
        check_msg(ButtplugDeviceMessageType::KiirooCmd)
      }
      ButtplugDeviceCommandMessageUnion::LinearCmd(_) => {
        check_msg(ButtplugDeviceMessageType::LinearCmd)
      }
      ButtplugDeviceCommandMessageUnion::RawReadCmd(_) => {
        check_msg(ButtplugDeviceMessageType::RawReadCmd)
      }
      ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(_) => {
        check_msg(ButtplugDeviceMessageType::RawSubscribeCmd)
      }
      ButtplugDeviceCommandMessageUnion::RawUnsubscribeCmd(_) => {
        check_msg(ButtplugDeviceMessageType::RawUnsubscribeCmd)
      }
      ButtplugDeviceCommandMessageUnion::RawWriteCmd(_) => {
        check_msg(ButtplugDeviceMessageType::RawWriteCmd)
      }
      ButtplugDeviceCommandMessageUnion::RotateCmd(_) => {
        check_msg(ButtplugDeviceMessageType::RotateCmd)
      }
      ButtplugDeviceCommandMessageUnion::RSSILevelCmd(_) => {
        check_msg(ButtplugDeviceMessageType::RSSILevelCmd)
      }
      ButtplugDeviceCommandMessageUnion::ScalarCmd(_) => {
        check_msg(ButtplugDeviceMessageType::ScalarCmd)
      }
      // We translate SingleMotorVibrateCmd into Vibrate, so this one is special.
      ButtplugDeviceCommandMessageUnion::SingleMotorVibrateCmd(_) => {
        check_msg(ButtplugDeviceMessageType::ScalarCmd)
      }
      ButtplugDeviceCommandMessageUnion::StopDeviceCmd(_) => {
        //check_msg(ButtplugDeviceMessageType::StopDeviceCmd)
        Ok(())
      }
      ButtplugDeviceCommandMessageUnion::VibrateCmd(_) => {
        check_msg(ButtplugDeviceMessageType::ScalarCmd)
      }
      ButtplugDeviceCommandMessageUnion::VorzeA10CycloneCmd(_) => {
        check_msg(ButtplugDeviceMessageType::VorzeA10CycloneCmd)
      }
      ButtplugDeviceCommandMessageUnion::SensorReadCmd(_) => {
        check_msg(ButtplugDeviceMessageType::SensorReadCmd)
      }
      ButtplugDeviceCommandMessageUnion::SensorSubscribeCmd(_) => {
        check_msg(ButtplugDeviceMessageType::SensorSubscribeCmd)
      }
      ButtplugDeviceCommandMessageUnion::SensorUnsubscribeCmd(_) => {
        check_msg(ButtplugDeviceMessageType::SensorUnsubscribeCmd)
      }
    }
    .map_err(|err| err.into())
  }

  // In order to not have to worry about id setting at the protocol level (this
  // should be taken care of in the server's device manager), we return server
  // messages but Buttplug errors.
  pub fn parse_message(
    &self,
    command_message: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugServerResultFuture {
    if let Err(err) = self.supports_message(&command_message) {
      return future::ready(Err(err)).boxed();
    }

    // If a handler implements handle message, bypass all of our parsing and let it do its own
    // thing. This should be a very rare thing.
    if self.handler.has_handle_message() {
      let fut = self.handle_generic_command_result(self.handler.handle_message(&command_message));
      return async move { fut.await }.boxed();
    }

    match command_message {
      // Raw messages
      ButtplugDeviceCommandMessageUnion::RawReadCmd(msg) => self.handle_raw_read_cmd(msg),
      ButtplugDeviceCommandMessageUnion::RawWriteCmd(msg) => self.handle_raw_write_cmd(msg),
      ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(msg) => self.handle_raw_subscribe_cmd(msg),
      ButtplugDeviceCommandMessageUnion::RawUnsubscribeCmd(msg) => {
        self.handle_raw_unsubscribe_cmd(msg)
      }
      // Sensor messages
      ButtplugDeviceCommandMessageUnion::SensorReadCmd(msg) => self.handle_sensor_read_cmd_v3(msg),
      ButtplugDeviceCommandMessageUnion::SensorSubscribeCmd(msg) => {
        self.handle_sensor_subscribe_cmd_v3(msg)
      }
      ButtplugDeviceCommandMessageUnion::SensorUnsubscribeCmd(msg) => {
        self.handle_sensor_unsubscribe_cmd_v3(msg)
      }
      // Actuator messages
      ButtplugDeviceCommandMessageUnion::ScalarCmd(msg) => self.handle_scalarcmd_v3(&msg),
      ButtplugDeviceCommandMessageUnion::RotateCmd(msg) => {
        let commands = match self
          .actuator_command_manager
          .update_rotation(&msg, self.handler.needs_full_command_set())
        {
          Ok(values) => values,
          Err(err) => return future::ready(Err(err)).boxed(),
        };
        self.handle_generic_command_result(self.handler.handle_rotate_cmd(&commands))
      }
      ButtplugDeviceCommandMessageUnion::LinearCmd(msg) => {
        self.handle_generic_command_result(self.handler.handle_linear_cmd(msg))
      }
      // Other generic messages
      ButtplugDeviceCommandMessageUnion::StopDeviceCmd(_) => self.handle_stop_device_cmd(),

      // V2 Message compatibility
      ButtplugDeviceCommandMessageUnion::VibrateCmd(msg) => self.handle_vibrate_cmd(msg),
      ButtplugDeviceCommandMessageUnion::BatteryLevelCmd(_) => self.handle_battery_level_cmd(),
      ButtplugDeviceCommandMessageUnion::RSSILevelCmd(_) => self.handle_rssi_level_cmd(),

      // V1 Message compatibility
      ButtplugDeviceCommandMessageUnion::SingleMotorVibrateCmd(msg) => {
        self.handle_single_motor_vibrate_cmd(msg)
      }
      ButtplugDeviceCommandMessageUnion::FleshlightLaunchFW12Cmd(msg) => {
        self.handle_generic_command_result(self.handler.handle_fleshlight_launch_fw12_cmd(msg))
      }
      ButtplugDeviceCommandMessageUnion::VorzeA10CycloneCmd(msg) => {
        self.handle_generic_command_result(self.handler.handle_vorze_a10_cyclone_cmd(msg))
      }
      ButtplugDeviceCommandMessageUnion::KiirooCmd(_) => future::ready(Err(
        ButtplugDeviceError::ProtocolNotImplemented("Being Lazy".to_owned()).into(),
      ))
      .boxed(),
    }
  }

  fn handle_scalarcmd_v3(&self, scalar_cmd: &ScalarCmd) -> ButtplugServerResultFuture {
    let scalar_features: Vec<usize> = self
      .definition
      .features()
      .iter()
      .enumerate()
      .filter(|(_, x)| {
        x.actuator().as_ref().is_some_and(|y| {
          y.messages()
            .contains(&message::ButtplugActuatorFeatureMessageType::ScalarCmd)
        })
      })
      .map(|(index, _)| index)
      .collect();

    let scalars_v4: Vec<ScalarSubcommandV4> = scalar_cmd
      .scalars()
      .iter()
      .map(|x| {
        ScalarSubcommandV4::new(
          scalar_features[x.index() as usize] as u32,
          x.scalar().clone(),
          x.actuator_type().clone(),
        )
      })
      .collect();

    let scalarcmd_v4 = ScalarCmdV4::new(scalar_cmd.device_index(), scalars_v4);
    self.handle_scalarcmd_v4(&scalarcmd_v4)
  }

  fn handle_scalarcmd_v4(&self, msg: &ScalarCmdV4) -> ButtplugServerResultFuture {
    for command in msg.scalars() {
      if command.feature_index() > self.definition.features().len() as u32 {
        return future::ready(Err(
          ButtplugDeviceError::DeviceFeatureIndexError(
            self.definition.features().len() as u32,
            command.feature_index(),
          )
          .into(),
        ))
        .boxed();
      }
      let feature_type =
        self.definition.features()[command.feature_index() as usize].feature_type();
      if *feature_type != command.actuator_type().into() {
        return future::ready(Err(
          ButtplugDeviceError::DeviceActuatorTypeMismatch(
            self.name(),
            command.actuator_type(),
            *feature_type,
          )
          .into(),
        ))
        .boxed();
      }
    }

    let commands = match self
      .actuator_command_manager
      .update_scalar(&msg, self.handler.needs_full_command_set())
    {
      Ok(values) => values,
      Err(err) => return future::ready(Err(err)).boxed(),
    };

    if commands.is_empty() {
      trace!("No commands generated for incoming device packet, skipping and returning success.");
      return future::ready(Ok(message::Ok::default().into())).boxed();
    }

    self.handle_generic_command_result(self.handler.handle_scalar_cmd(&commands))
  }

  fn handle_hardware_commands(&self, commands: Vec<HardwareCommand>) -> ButtplugServerResultFuture {
    let hardware = self.hardware.clone();
    let keepalive_type = self.handler.keepalive_strategy();
    let keepalive_packet = self.keepalive_packet.clone();
    async move {
      // Run commands in order, otherwise we may end up sending out of order. This may take a while,
      // but it's what 99% of protocols expect. If they want something else, they can implement it
      // themselves.
      //
      // If anything errors out, just bail on the command series. This most likely means the device
      // disconnected.
      for command in commands {
        hardware.parse_message(&command).await?;
        if hardware.requires_keepalive()
          && matches!(
            keepalive_type,
            ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
          )
        {
          if let HardwareCommand::Write(command) = command {
            *keepalive_packet.write().await = Some(command);
          }
        }
      }
      Ok(message::Ok::default().into())
    }
    .boxed()
  }

  fn handle_generic_command_result(
    &self,
    command_result: Result<Vec<HardwareCommand>, ButtplugDeviceError>,
  ) -> ButtplugServerResultFuture {
    let hardware_commands = match command_result {
      Ok(commands) => commands,
      Err(err) => return future::ready(Err(err.into())).boxed(),
    };

    self.handle_hardware_commands(hardware_commands)
  }

  fn handle_stop_device_cmd(&self) -> ButtplugServerResultFuture {
    let commands = self.actuator_command_manager.stop_commands();
    let mut fut_vec = vec![];
    commands
      .iter()
      .for_each(|msg| fut_vec.push(self.parse_message(msg.clone())));
    async move {
      for fut in fut_vec {
        fut.await?;
      }
      Ok(message::Ok::default().into())
    }
    .boxed()
  }

  fn check_sensor_command(
    &self,
    feature_index: &u32,
    sensor_type: &SensorType,
  ) -> Result<(), ButtplugDeviceError> {
    if *feature_index > self.definition.features().len() as u32 {
      return Err(ButtplugDeviceError::DeviceSensorIndexError(
        self.definition.features().len() as u32,
        *feature_index,
      ));
    }
    let feature_type = self.definition.features()[*feature_index as usize].feature_type();
    if *feature_type != FeatureType::from(*sensor_type) {
      Err(ButtplugDeviceError::DeviceSensorTypeMismatch(
        *feature_index,
        *sensor_type,
        *feature_type,
      ))
    } else {
      Ok(())
    }
  }

  fn handle_sensor_read_cmd_v3(&self, message: SensorReadCmd) -> ButtplugServerResultFuture {
    let sensor_features: Vec<usize> = self
      .definition
      .features()
      .iter()
      .enumerate()
      .filter(|(_, x)| {
        x.sensor().as_ref().is_some_and(|y| {
          y.messages()
            .contains(&message::ButtplugSensorFeatureMessageType::SensorReadCmd)
        })
      })
      .map(|(index, _)| index)
      .collect();
    let sensor_feature_index = sensor_features[*message.sensor_index() as usize] as u32;

    let sensor_read_v4 = SensorReadCmdV4::new(
      message.device_index(),
      sensor_feature_index,
      *message.sensor_type(),
    );

    let read_fut = self.handle_sensor_read_cmd_v4(sensor_read_v4);
    async move {
      read_fut.await.map(|res| {
        SensorReading::new(
          message.device_index(),
          *message.sensor_index(),
          *message.sensor_type(),
          res.data().clone(),
        )
        .into()
      })
    }
    .boxed()
  }

  fn handle_sensor_read_cmd_v4(
    &self,
    message: message::SensorReadCmdV4,
  ) -> BoxFuture<'static, Result<SensorReadingV4, ButtplugError>> {
    let result = self.check_sensor_command(message.feature_index(), message.sensor_type());
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      result?;
      handler
        .handle_sensor_read_cmd(device, message)
        .await
        .map_err(|e| e.into())
    }
    .boxed()
  }

  fn handle_sensor_subscribe_cmd_v3(
    &self,
    message: SensorSubscribeCmd,
  ) -> ButtplugServerResultFuture {
    let sensor_features: Vec<usize> = self
      .definition
      .features()
      .iter()
      .enumerate()
      .filter(|(_, x)| {
        x.sensor().as_ref().is_some_and(|y| {
          y.messages()
            .contains(&message::ButtplugSensorFeatureMessageType::SensorSubscribeCmd)
        })
      })
      .map(|(index, _)| index)
      .collect();
    let sensor_feature_index = sensor_features[*message.sensor_index() as usize] as u32;

    let sensor_subscribe_v4 = SensorSubscribeCmdV4::new(
      message.device_index(),
      sensor_feature_index,
      *message.sensor_type(),
    );
    self.handle_sensor_subscribe_cmd_v4(sensor_subscribe_v4)
  }

  fn handle_sensor_subscribe_cmd_v4(
    &self,
    message: message::SensorSubscribeCmdV4,
  ) -> ButtplugServerResultFuture {
    let result = self.check_sensor_command(message.feature_index(), message.sensor_type());
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      result?;
      handler
        .handle_sensor_subscribe_cmd(device, message)
        .await
        .map_err(|e| e.into())
    }
    .boxed()
  }

  fn handle_sensor_unsubscribe_cmd_v3(
    &self,
    message: message::SensorUnsubscribeCmd,
  ) -> ButtplugServerResultFuture {
    let sensor_features: Vec<usize> = self
      .definition
      .features()
      .iter()
      .enumerate()
      .filter(|(_, x)| {
        x.sensor().as_ref().is_some_and(|y| {
          y.messages()
            .contains(&message::ButtplugSensorFeatureMessageType::SensorSubscribeCmd)
        })
      })
      .map(|(index, _)| index)
      .collect();
    let sensor_feature_index = sensor_features[*message.sensor_index() as usize] as u32;

    let sensor_unsubscribe_v4 = SensorUnsubscribeCmdV4::new(
      message.device_index(),
      sensor_feature_index,
      *message.sensor_type(),
    );
    self.handle_sensor_unsubscribe_cmd_v4(sensor_unsubscribe_v4)
  }

  fn handle_sensor_unsubscribe_cmd_v4(
    &self,
    message: message::SensorUnsubscribeCmdV4,
  ) -> ButtplugServerResultFuture {
    let result = self.check_sensor_command(message.feature_index(), message.sensor_type());
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      result?;
      handler
        .handle_sensor_unsubscribe_cmd(device, message)
        .await
        .map_err(|e| e.into())
    }
    .boxed()
  }

  fn handle_vibrate_cmd(&self, message: VibrateCmd) -> ButtplugServerResultFuture {
    let vibrate_features: Vec<usize> = self
      .definition
      .features()
      .iter()
      .enumerate()
      .filter(|(_, x)| {
        *x.feature_type() == FeatureType::Vibrate
          && x.actuator().as_ref().is_some_and(|y| {
            y.messages()
              .contains(&message::ButtplugActuatorFeatureMessageType::ScalarCmd)
          })
      })
      .map(|(index, _)| index)
      .collect();

    let cmds: Vec<ScalarSubcommandV4> = message
      .speeds()
      .iter()
      .map(|x| {
        ScalarSubcommandV4::new(
          vibrate_features[x.index() as usize] as u32,
          x.speed(),
          ActuatorType::Vibrate,
        )
      })
      .collect();

    if cmds.is_empty() {
      ButtplugDeviceError::ProtocolRequirementError(format!(
        "{} has no vibrating features.",
        self.name()
      ))
      .into()
    } else {
      let mut vibrate_cmd = ScalarCmdV4::new(message.device_index(), cmds);
      vibrate_cmd.set_id(message.id());
      self.handle_scalarcmd_v4(&vibrate_cmd)
    }
  }

  fn handle_single_motor_vibrate_cmd(
    &self,
    message: message::SingleMotorVibrateCmd,
  ) -> ButtplugServerResultFuture {
    let vibrate_features: Vec<usize> = self
      .definition
      .features()
      .iter()
      .enumerate()
      .filter(|(_, x)| {
        *x.feature_type() == FeatureType::Vibrate
          && x.actuator().as_ref().is_some_and(|y| {
            y.messages()
              .contains(&message::ButtplugActuatorFeatureMessageType::ScalarCmd)
          })
      })
      .map(|(index, _)| index)
      .collect();

    let cmds: Vec<ScalarSubcommandV4> = vibrate_features
      .iter()
      .map(|x| ScalarSubcommandV4::new(*x as u32, message.speed(), ActuatorType::Vibrate))
      .collect();

    if cmds.is_empty() {
      ButtplugDeviceError::ProtocolRequirementError(format!(
        "{} has no vibrating features.",
        self.name()
      ))
      .into()
    } else {
      let mut vibrate_cmd = ScalarCmdV4::new(message.device_index(), cmds);
      vibrate_cmd.set_id(message.id());
      self.handle_scalarcmd_v4(&vibrate_cmd)
    }
  }

  fn handle_raw_write_cmd(&self, message: message::RawWriteCmd) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = self.hardware.write_value(&message.into());
    async move {
      fut
        .await
        .map(|_| message::Ok::new(id).into())
        .map_err(|err| err.into())
    }
    .boxed()
  }

  fn handle_raw_read_cmd(&self, message: message::RawReadCmd) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = self.hardware.read_value(&message.into());
    async move {
      fut
        .await
        .map(|msg| {
          let mut raw_msg: RawReading = msg.into();
          raw_msg.set_id(id);
          raw_msg.into()
        })
        .map_err(|err| err.into())
    }
    .boxed()
  }

  fn handle_raw_unsubscribe_cmd(
    &self,
    message: message::RawUnsubscribeCmd,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let endpoint = message.endpoint();
    let fut = self.hardware.unsubscribe(&message.into());
    let raw_endpoints = self.raw_subscribed_endpoints.clone();
    async move {
      if !raw_endpoints.contains(&endpoint) {
        return Ok(message::Ok::new(id).into());
      }
      let result = fut
        .await
        .map(|_| message::Ok::new(id).into())
        .map_err(|err| err.into());
      raw_endpoints.remove(&endpoint);
      result
    }
    .boxed()
  }

  fn handle_raw_subscribe_cmd(
    &self,
    message: message::RawSubscribeCmd,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let endpoint = message.endpoint();
    let fut = self.hardware.subscribe(&message.into());
    let raw_endpoints = self.raw_subscribed_endpoints.clone();
    async move {
      if raw_endpoints.contains(&endpoint) {
        return Ok(message::Ok::new(id).into());
      }
      let result = fut
        .await
        .map(|_| message::Ok::new(id).into())
        .map_err(|err| err.into());
      raw_endpoints.insert(endpoint);
      result
    }
    .boxed()
  }

  fn handle_battery_level_cmd(&self) -> ButtplugServerResultFuture {
    if let Some((index, battery_feature)) =
      self
        .definition
        .features()
        .iter()
        .enumerate()
        .find(|(_, x)| {
          *x.feature_type() == FeatureType::Battery
            && x.sensor().as_ref().is_some_and(|y| {
              y.messages()
                .contains(&message::ButtplugSensorFeatureMessageType::SensorReadCmd)
            })
        })
    {
      let sensor_read_msg = SensorReadCmdV4::new(0, index as u32, SensorType::Battery);
      let sensor_read = self.handle_sensor_read_cmd_v4(sensor_read_msg);
      let sensor_range_end = *battery_feature.sensor().as_ref().unwrap().value_range()[0].end();
      return async move {
        let reading = sensor_read.await?;
        if reading.sensor_type() == SensorType::Battery {
          Ok(BatteryLevelReading::new(0, reading.data()[0] as f64 / sensor_range_end as f64).into())
        } else {
          Err(ButtplugError::ButtplugDeviceError(
            ButtplugDeviceError::ProtocolSensorNotSupported(SensorType::Battery),
          ))
        }
      }
      .boxed();
    }
    future::ready(Err(ButtplugError::ButtplugDeviceError(
      ButtplugDeviceError::ProtocolSensorNotSupported(SensorType::Battery),
    )))
    .boxed()
  }

  fn handle_rssi_level_cmd(&self) -> ButtplugServerResultFuture {
    if let Some((index, _)) = self
      .definition
      .features()
      .iter()
      .enumerate()
      .find(|(_, x)| {
        *x.feature_type() == FeatureType::RSSI
          && x.sensor().as_ref().is_some_and(|y| {
            y.messages()
              .contains(&message::ButtplugSensorFeatureMessageType::SensorReadCmd)
          })
      })
    {
      let sensor_read_msg = SensorReadCmdV4::new(0, index as u32, SensorType::RSSI);
      let sensor_read = self.handle_sensor_read_cmd_v4(sensor_read_msg);
      return async move {
        let reading = sensor_read.await?;
        if reading.sensor_type() == SensorType::RSSI {
          Ok(RSSILevelReading::new(0, reading.data()[0]).into())
        } else {
          Err(ButtplugError::ButtplugDeviceError(
            ButtplugDeviceError::ProtocolSensorNotSupported(SensorType::RSSI),
          ))
        }
      }
      .boxed();
    }
    future::ready(Err(ButtplugError::ButtplugDeviceError(
      ButtplugDeviceError::ProtocolSensorNotSupported(SensorType::RSSI),
    )))
    .boxed()
  }
}
