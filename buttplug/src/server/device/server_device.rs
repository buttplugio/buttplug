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
      ButtplugMessage,
      ButtplugServerMessageV4,
      Endpoint,
      FeatureType,
      RawReadingV2,
      RawSubscribeCmdV2,
      SensorType,
    },
    ButtplugResultFuture,
  },
  server::{
    device::{
      configuration::DeviceConfigurationManager,
      hardware::{Hardware, HardwareCommand, HardwareConnector, HardwareEvent},
      protocol::ProtocolHandler,
    },
    message::{
      checked_level_cmd::CheckedLevelCmdV4, checked_sensor_read_cmd::CheckedSensorReadCmdV4, checked_sensor_subscribe_cmd::CheckedSensorSubscribeCmdV4, checked_sensor_unsubscribe_cmd::CheckedSensorUnsubscribeCmdV4, legacy_device_attributes::LegacyDeviceAttributes, spec_enums::ButtplugDeviceCommandMessageUnion, ButtplugDeviceMessageType, ButtplugServerDeviceMessage
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
use uuid::Uuid;

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
  #[getset(get = "pub")]
  legacy_attributes: LegacyDeviceAttributes,
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
      .initialize(hardware.clone(), &attrs.clone())
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
                if let Err(e) = hardware.write_value(packet).await {
                  warn!("Error writing keepalive packet: {:?}", e);
                  break;
                }
              }
              ProtocolKeepaliveStrategy::RepeatLastPacketStrategy => {
                if let Some(packet) = &*keepalive_packet.read().await {
                  if let Err(e) = hardware.write_value(packet).await {
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
      // Generating legacy attributes is cheap, just do it right when we create the device.
      legacy_attributes: LegacyDeviceAttributes::new(definition.features()),
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
        RawSubscribeCmdV2::new(1, Endpoint::Tx),
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
                ButtplugServerDeviceMessage::RawReading(RawReadingV2::new(0, endpoint, data)),
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
        .ok_or(ButtplugDeviceError::MessageNotSupported(
          msg_type.to_string(),
        ))
    };

    match message {
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
      ButtplugDeviceCommandMessageUnion::LevelCmd(_) => {
        check_msg(ButtplugDeviceMessageType::LevelCmd)
      }
      ButtplugDeviceCommandMessageUnion::StopDeviceCmd(_) => {
        //check_msg(ButtplugDeviceMessageType::StopDeviceCmd)
        Ok(())
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
      return fut.boxed();
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
      ButtplugDeviceCommandMessageUnion::SensorReadCmd(msg) => self.handle_sensor_read_cmd_v4(msg),
      ButtplugDeviceCommandMessageUnion::SensorSubscribeCmd(msg) => {
        self.handle_sensor_subscribe_cmd_v4(msg)
      }
      ButtplugDeviceCommandMessageUnion::SensorUnsubscribeCmd(msg) => {
        self.handle_sensor_unsubscribe_cmd_v4(msg)
      }
      // Actuator messages
      ButtplugDeviceCommandMessageUnion::LevelCmd(msg) => self.handle_levelcmd_v4(&msg),
      /*
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
      */
      ButtplugDeviceCommandMessageUnion::LinearCmd(msg) => {
        self.handle_generic_command_result(self.handler.handle_linear_cmd(msg))
      }
      // Other generic messages
      ButtplugDeviceCommandMessageUnion::StopDeviceCmd(_) => self.handle_stop_device_cmd(),
    }
  }

  fn handle_levelcmd_v4(&self, msg: &CheckedLevelCmdV4) -> ButtplugServerResultFuture {
    let commands = match self
      .actuator_command_manager
      .update_level(&msg, self.handler.needs_full_command_set())
    {
      Ok(values) => values,
      Err(err) => return future::ready(Err(err)).boxed(),
    };

    if commands.is_empty() || commands.iter().filter(|x| x.is_some()).count() == 0 {
      trace!("No commands generated for incoming device packet, skipping and returning success.");
      return future::ready(Ok(message::OkV0::default().into())).boxed();
    }
    self.handle_generic_command_result(
      self.handler.handle_scalar_cmd(
        &commands
          .iter()
          .map(|x| {
            if let Some((y, z)) = x {
              Some((*y, *z))
            } else {
              None
            }
          })
          .collect::<Vec<Option<(ActuatorType, i32)>>>(),
      ),
    )
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
      Ok(message::OkV0::default().into())
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
      Ok(message::OkV0::default().into())
    }
    .boxed()
  }

  fn check_sensor_command(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    sensor_type: SensorType,
  ) -> Result<(), ButtplugDeviceError> {
    if let Some(feature) = self
      .definition
      .features()
      .iter()
      .find(|x| *x.id() == feature_id)
    {
      if *feature.feature_type() == FeatureType::from(sensor_type) {
        Ok(())
      } else {
        Err(ButtplugDeviceError::DeviceSensorTypeMismatch(
          feature_id.to_string(),
          sensor_type,
          *feature.feature_type(),
        ))
      }
    } else {
      Err(ButtplugDeviceError::DeviceSensorIndexError(
        self.definition.features().len() as u32,
        feature_index,
      ))
    }
  }

  fn handle_sensor_read_cmd_v4(
    &self,
    message: CheckedSensorReadCmdV4,
  ) -> BoxFuture<'static, Result<ButtplugServerMessageV4, ButtplugError>> {
    let result = self.check_sensor_command(
      message.feature_index(),
      message.feature_id(),
      message.sensor_type(),
    );
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      result?;
      handler
        .handle_sensor_read_cmd(device, &message)
        .await
        .map_err(|e| e.into())
        .map(|e| e.into())
    }
    .boxed()
  }

  fn handle_sensor_subscribe_cmd_v4(
    &self,
    message: CheckedSensorSubscribeCmdV4,
  ) -> ButtplugServerResultFuture {
    let result = self.check_sensor_command(
      message.feature_index(),
      message.feature_id(),
      message.sensor_type(),
    );
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      result?;
      handler
        .handle_sensor_subscribe_cmd(device, &message)
        .await
        .map(|_| message::OkV0::new(message.id()).into())
        .map_err(|e| e.into())
    }
    .boxed()
  }

  fn handle_sensor_unsubscribe_cmd_v4(
    &self,
    message: CheckedSensorUnsubscribeCmdV4,
  ) -> ButtplugServerResultFuture {
    let result = self.check_sensor_command(
      message.feature_index(),
      message.feature_id(),
      message.sensor_type(),
    );
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      result?;
      handler
        .handle_sensor_unsubscribe_cmd(device, &message)
        .await
        .map(|_| message::OkV0::new(message.id()).into())
        .map_err(|e| e.into())
    }
    .boxed()
  }

  fn handle_raw_write_cmd(&self, message: message::RawWriteCmdV2) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = self.hardware.write_value(&message.into());
    async move {
      fut
        .await
        .map(|_| message::OkV0::new(id).into())
        .map_err(|err| err.into())
    }
    .boxed()
  }

  fn handle_raw_read_cmd(&self, message: message::RawReadCmdV2) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = self.hardware.read_value(&message.into());
    async move {
      fut
        .await
        .map(|msg| {
          let mut raw_msg: RawReadingV2 = msg.into();
          raw_msg.set_id(id);
          raw_msg.into()
        })
        .map_err(|err| err.into())
    }
    .boxed()
  }

  fn handle_raw_unsubscribe_cmd(
    &self,
    message: message::RawUnsubscribeCmdV2,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let endpoint = message.endpoint();
    let fut = self.hardware.unsubscribe(&message.into());
    let raw_endpoints = self.raw_subscribed_endpoints.clone();
    async move {
      if !raw_endpoints.contains(&endpoint) {
        return Ok(message::OkV0::new(id).into());
      }
      let result = fut
        .await
        .map(|_| message::OkV0::new(id).into())
        .map_err(|err| err.into());
      raw_endpoints.remove(&endpoint);
      result
    }
    .boxed()
  }

  fn handle_raw_subscribe_cmd(
    &self,
    message: message::RawSubscribeCmdV2,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let endpoint = message.endpoint();
    let fut = self.hardware.subscribe(&message.into());
    let raw_endpoints = self.raw_subscribed_endpoints.clone();
    async move {
      if raw_endpoints.contains(&endpoint) {
        return Ok(message::OkV0::new(id).into());
      }
      let result = fut
        .await
        .map(|_| message::OkV0::new(id).into())
        .map_err(|err| err.into());
      raw_endpoints.insert(endpoint);
      result
    }
    .boxed()
  }
}
