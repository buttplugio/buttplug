// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Device Handle - Owns device state and handles communication
//!
//! DeviceHandle provides the interface for sending commands to devices.
//! It owns the device state directly and handles all command processing.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use buttplug_core::{
  ButtplugResultFuture,
  errors::{ButtplugDeviceError, ButtplugError},
  message::{
    self, ButtplugMessage, ButtplugServerMessageV4, DeviceFeature, DeviceMessageInfoV4,
    InputCommandType, InputType, OutputType, OutputValue, StopDeviceCmdV4,
  },
  util::{async_manager, stream::convert_broadcast_receiver_to_stream},
};
use buttplug_server_device_config::{
  DeviceConfigurationManager, ServerDeviceDefinition, UserDeviceIdentifier,
};
use dashmap::DashMap;
use futures::future::{self, BoxFuture, FutureExt};
use tokio::sync::{mpsc::{channel, Sender}, oneshot};
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::{
  ButtplugServerResultFuture,
  message::{
    ButtplugServerDeviceMessage,
    checked_input_cmd::CheckedInputCmdV4,
    checked_output_cmd::CheckedOutputCmdV4,
    server_device_attributes::ServerDeviceAttributes,
    spec_enums::ButtplugDeviceCommandMessageUnionV4,
  },
};

use super::{
  InternalDeviceEvent,
  device_task::{spawn_device_task, DeviceTaskConfig},
  hardware::{Hardware, HardwareCommand, HardwareConnector, HardwareEvent},
  protocol::{ProtocolHandler, ProtocolKeepaliveStrategy, ProtocolSpecializer},
};

/// Commands that can be sent to a device through its handle.
///
/// Each command variant includes a oneshot channel for returning the result
/// back to the caller.
#[derive(Debug)]
pub enum DeviceCommand {
  /// Output command (vibrate, rotate, oscillate, etc.)
  Output {
    cmd: CheckedOutputCmdV4,
    response: oneshot::Sender<Result<(), ButtplugError>>,
  },
  /// Input command (read sensor, subscribe/unsubscribe, etc.)
  Input {
    cmd: CheckedInputCmdV4,
    response: oneshot::Sender<Result<(), ButtplugError>>,
  },
  /// Stop all device outputs and optionally unsubscribe from inputs
  Stop {
    stop_outputs: bool,
    stop_inputs: bool,
    response: oneshot::Sender<Result<(), ButtplugError>>,
  },
  /// Disconnect the device
  Disconnect,
}

/// Events emitted by devices
#[derive(Debug)]
pub enum DeviceEvent {
  Notification(UserDeviceIdentifier, ButtplugServerDeviceMessage),
  Disconnected(UserDeviceIdentifier),
}

/// Handle for communicating with a device.
///
/// DeviceHandle owns the device state directly and handles all command
/// processing. It is cheap to clone and can be safely shared across tasks.
#[derive(Clone)]
pub struct DeviceHandle {
  hardware: Arc<Hardware>,
  handler: Arc<dyn ProtocolHandler>,
  definition: ServerDeviceDefinition,
  identifier: UserDeviceIdentifier,
  legacy_attributes: ServerDeviceAttributes,
  last_output_command: Arc<DashMap<Uuid, CheckedOutputCmdV4>>,
  stop_commands: Arc<Vec<ButtplugDeviceCommandMessageUnionV4>>,
  internal_hw_msg_sender: Sender<Vec<HardwareCommand>>,
}

impl DeviceHandle {
  /// Create a new DeviceHandle with direct ownership of device state
  pub(crate) fn new(
    hardware: Arc<Hardware>,
    handler: Arc<dyn ProtocolHandler>,
    definition: ServerDeviceDefinition,
    identifier: UserDeviceIdentifier,
    stop_commands: Vec<ButtplugDeviceCommandMessageUnionV4>,
    internal_hw_msg_sender: Sender<Vec<HardwareCommand>>,
  ) -> Self {
    Self {
      hardware,
      handler,
      legacy_attributes: ServerDeviceAttributes::new(definition.features()),
      definition,
      identifier,
      last_output_command: Arc::new(DashMap::new()),
      stop_commands: Arc::new(stop_commands),
      internal_hw_msg_sender,
    }
  }

  /// Get the device's unique identifier
  pub fn identifier(&self) -> &UserDeviceIdentifier {
    &self.identifier
  }

  /// Get the device's name
  pub fn name(&self) -> String {
    self.definition.name().to_owned()
  }

  /// Get the device's definition (contains features, display name, etc.)
  pub fn definition(&self) -> &ServerDeviceDefinition {
    &self.definition
  }

  /// Get the device's legacy attributes (for older API compatibility)
  pub fn legacy_attributes(&self) -> &ServerDeviceAttributes {
    &self.legacy_attributes
  }

  /// Get the device as a DeviceMessageInfoV4 for protocol messages
  pub fn as_device_message_info(&self, index: u32) -> DeviceMessageInfoV4 {
    DeviceMessageInfoV4::new(
      index,
      &self.name(),
      self.definition.display_name(),
      100,
      &self
        .definition
        .features()
        .values()
        .map(|x| (x.index(), x.as_device_feature().expect("Infallible")))
        .filter(|(_, x)| x.output().as_ref().is_some() || x.input().as_ref().is_some())
        .collect::<BTreeMap<u32, DeviceFeature>>(),
    )
  }

  /// Parse and handle a command message for this device
  pub fn parse_message(
    &self,
    command_message: ButtplugDeviceCommandMessageUnionV4,
  ) -> ButtplugServerResultFuture {
    match &command_message {
      // Input messages
      ButtplugDeviceCommandMessageUnionV4::InputCmd(msg) => {
        let fut = self.handle_input_cmd(msg);
        let msg_id = msg.id();
        async move {
          let mut msg = fut.await?;
          msg.set_id(msg_id);
          Ok(msg)
        }
        .boxed()
      }
      // Actuator messages
      ButtplugDeviceCommandMessageUnionV4::OutputCmd(msg) => self.handle_outputcmd_v4(msg),
      ButtplugDeviceCommandMessageUnionV4::OutputVecCmd(msg) => {
        let mut futs = vec![];
        let msg_id = msg.id();
        for m in msg.value_vec() {
          futs.push(self.handle_outputcmd_v4(m))
        }
        async move {
          for f in futs {
            f.await?;
          }
          Ok(message::OkV0::new(msg_id).into())
        }
        .boxed()
      }
      // Other generic messages
      ButtplugDeviceCommandMessageUnionV4::StopDeviceCmd(msg) => self.handle_stop_device_cmd(msg),
    }
  }

  /// Disconnect from the device
  pub fn disconnect(&self) -> ButtplugResultFuture {
    let fut = self.hardware.disconnect();
    async move { fut.await.map_err(|err| err.into()) }.boxed()
  }

  /// Get the event stream for this device (disconnections, notifications)
  pub fn event_stream(&self) -> impl futures::Stream<Item = DeviceEvent> + Send + use<> {
    let identifier = self.identifier.clone();
    let hardware_stream = convert_broadcast_receiver_to_stream(self.hardware.event_stream())
      .filter_map(move |hardware_event| {
        let id = identifier.clone();
        match hardware_event {
          HardwareEvent::Disconnected(_) => Some(DeviceEvent::Disconnected(id)),
          HardwareEvent::Notification(_address, _endpoint, _data) => {
            // TODO Does this still need to be here? Does this need to be routed to the protocol it's part of?
            None
          }
        }
      });

    let identifier = self.identifier.clone();
    let handler_mapped_stream = self.handler.event_stream().map(move |incoming_message| {
      let id = identifier.clone();
      DeviceEvent::Notification(id, incoming_message)
    });
    hardware_stream.merge(handler_mapped_stream)
  }

  // --- Private command handling methods ---

  fn handle_outputcmd_v4(&self, msg: &CheckedOutputCmdV4) -> ButtplugServerResultFuture {
    if let Some(last_msg) = self.last_output_command.get(&msg.feature_id())
      && *last_msg == *msg
    {
      trace!("No commands generated for incoming device packet, skipping and returning success.");
      return future::ready(Ok(message::OkV0::default().into())).boxed();
    }
    self
      .last_output_command
      .insert(msg.feature_id(), msg.clone());
    self.handle_generic_command_result(self.handler.handle_output_cmd(msg))
  }

  fn handle_hardware_commands(&self, commands: Vec<HardwareCommand>) -> ButtplugServerResultFuture {
    let sender = self.internal_hw_msg_sender.clone();
    async move {
      let _ = sender.send(commands).await;
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

  fn handle_stop_device_cmd(&self, msg: &StopDeviceCmdV4) -> ButtplugServerResultFuture {
    let mut fut_vec = vec![];
    if msg.outputs() {
      self
        .stop_commands
        .iter()
        .for_each(|msg| fut_vec.push(self.parse_message(msg.clone())));
    }
    if msg.inputs() {
      self.definition.features().iter().for_each(|(i, f)| {
        if let Some(inputs) = f.input() {
          if inputs.can_subscribe() {
            fut_vec.push(
              self.parse_message(ButtplugDeviceCommandMessageUnionV4::InputCmd(
                CheckedInputCmdV4::new(
                  1,
                  self.definition.index(),
                  *i,
                  InputType::Unknown,
                  InputCommandType::Unsubscribe,
                  f.id(),
                ),
              )),
            );
          }
        }
      });
    }
    async move {
      for fut in fut_vec {
        fut.await?;
      }
      Ok(message::OkV0::default().into())
    }
    .boxed()
  }

  fn handle_input_cmd(
    &self,
    message: &CheckedInputCmdV4,
  ) -> BoxFuture<'static, Result<ButtplugServerMessageV4, ButtplugError>> {
    match message.input_command() {
      InputCommandType::Read => self.handle_input_read_cmd(
        message.device_index(),
        message.feature_index(),
        message.feature_id(),
        message.input_type(),
      ),
      InputCommandType::Subscribe => self.handle_input_subscribe_cmd(
        message.device_index(),
        message.feature_index(),
        message.feature_id(),
        message.input_type(),
      ),
      InputCommandType::Unsubscribe => self.handle_input_unsubscribe_cmd(
        message.feature_index(),
        message.feature_id(),
        message.input_type(),
      ),
    }
  }

  fn handle_input_read_cmd(
    &self,
    device_index: u32,
    feature_index: u32,
    feature_id: Uuid,
    input_type: InputType,
  ) -> BoxFuture<'static, Result<ButtplugServerMessageV4, ButtplugError>> {
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      handler
        .handle_input_read_cmd(device_index, device, feature_index, feature_id, input_type)
        .await
        .map_err(|e| e.into())
        .map(|e| e.into())
    }
    .boxed()
  }

  fn handle_input_subscribe_cmd(
    &self,
    device_index: u32,
    feature_index: u32,
    feature_id: Uuid,
    input_type: InputType,
  ) -> ButtplugServerResultFuture {
    info!("Handling input subscribe command");
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      handler
        .handle_input_subscribe_cmd(device_index, device, feature_index, feature_id, input_type)
        .await
        .map(|_| message::OkV0::new(1).into())
        .map_err(|e| e.into())
    }
    .boxed()
  }

  fn handle_input_unsubscribe_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    input_type: InputType,
  ) -> ButtplugServerResultFuture {
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      handler
        .handle_input_unsubscribe_cmd(device, feature_index, feature_id, input_type)
        .await
        .map(|_| message::OkV0::new(1).into())
        .map_err(|e| e.into())
    }
    .boxed()
  }
}

impl std::fmt::Debug for DeviceHandle {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("DeviceHandle")
      .field("identifier", &self.identifier)
      .field("name", &self.name())
      .finish()
  }
}

/// Build a DeviceHandle from hardware connectors and protocol specializers.
///
/// This function:
/// 1. Connects to the hardware
/// 2. Specializes it for the matched protocol
/// 3. Initializes the protocol handler
/// 4. Spawns the device communication task
/// 5. Spawns the device event forwarding task
/// 6. Returns a DeviceHandle for interacting with the device
pub(super) async fn build_device_handle(
  device_config_manager: Arc<DeviceConfigurationManager>,
  mut hardware_connector: Box<dyn HardwareConnector>,
  protocol_specializers: Vec<ProtocolSpecializer>,
  device_event_sender: tokio::sync::mpsc::Sender<InternalDeviceEvent>,
) -> Result<DeviceHandle, ButtplugDeviceError> {
  // At this point, we know we've got hardware that is waiting to connect, and enough protocol
  // info to actually do something after we connect. So go ahead and connect.
  trace!("Connecting to {:?}", hardware_connector);
  let mut hardware_specializer = hardware_connector.connect().await?;

  // We can't run these in parallel because we need to only accept one specializer.
  let mut protocol_identifier = None;
  let mut hardware_out = None;
  for protocol_specializer in protocol_specializers {
    match hardware_specializer
      .specialize(protocol_specializer.specifiers())
      .await
    {
      Ok(specialized_hardware) => {
        protocol_identifier = Some(protocol_specializer.identify());
        hardware_out = Some(specialized_hardware);
        break;
      }
      Err(e) => {
        error!("{:?}", e.to_string());
      }
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
  let definition = if let Some(attrs) = device_config_manager.device_definition(&identifier) {
    attrs
  } else {
    return Err(ButtplugDeviceError::DeviceConfigurationError(format!(
      "No protocols with viable protocol attributes for hardware {identifier:?}."
    )));
  };

  // Build the protocol handler
  let handler = protocol_initializer
    .initialize(hardware.clone(), &definition)
    .await?;

  let requires_keepalive = hardware.requires_keepalive();
  let strategy = handler.keepalive_strategy();

  // Create the hardware command channel and spawn the device task
  let (internal_hw_msg_sender, internal_hw_msg_recv) = channel::<Vec<HardwareCommand>>(1024);

  let device_wait_duration = if let Some(gap) = definition.message_gap_ms() {
    Some(Duration::from_millis(gap as u64))
  } else {
    hardware.message_gap()
  };

  spawn_device_task(
    hardware.clone(),
    handler.clone(),
    DeviceTaskConfig {
      message_gap: device_wait_duration,
      requires_keepalive: hardware.requires_keepalive(),
      keepalive_strategy: handler.keepalive_strategy(),
    },
    internal_hw_msg_recv,
  );

  // Generate stop commands for this device
  let mut stop_commands: Vec<ButtplugDeviceCommandMessageUnionV4> = vec![];
  for feature in definition.features().values() {
    if let Some(output_map) = feature.output() {
      for actuator_type in output_map.output_types() {
        let mut stop_cmd = |actuator_cmd| {
          stop_commands.push(
            CheckedOutputCmdV4::new(1, 0, feature.index(), feature.id(), actuator_cmd).into(),
          );
        };

        // Break out of these if one is found, we only need 1 stop message per output.
        match actuator_type {
          OutputType::Constrict => {
            stop_cmd(message::OutputCommand::Constrict(OutputValue::new(0)));
            break;
          }
          OutputType::Temperature => {
            stop_cmd(message::OutputCommand::Temperature(OutputValue::new(0)));
            break;
          }
          OutputType::Spray => {
            stop_cmd(message::OutputCommand::Spray(OutputValue::new(0)));
            break;
          }
          OutputType::Led => {
            stop_cmd(message::OutputCommand::Led(OutputValue::new(0)));
            break;
          }
          OutputType::Oscillate => {
            stop_cmd(message::OutputCommand::Oscillate(OutputValue::new(0)));
            break;
          }
          OutputType::Rotate => {
            stop_cmd(message::OutputCommand::Rotate(OutputValue::new(0)));
            break;
          }
          OutputType::Vibrate => {
            stop_cmd(message::OutputCommand::Vibrate(OutputValue::new(0)));
            break;
          }
          _ => {
            // There's not much we can do about position or position w/ duration, so just continue on
            continue;
          }
        }
      }
    }
  }

  // Create the DeviceHandle
  let device_handle = DeviceHandle::new(
    hardware,
    handler,
    definition.clone(),
    identifier,
    stop_commands,
    internal_hw_msg_sender,
  );

  // If we need a keepalive with a packet replay, set this up via stopping the device on connect.
  if ((requires_keepalive
    && matches!(
      strategy,
      ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
    ))
    || matches!(
      strategy,
      ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(_)
    ))
    && let Err(e) = device_handle
      .parse_message(StopDeviceCmdV4::new(0, true, true).into())
      .await
  {
    return Err(ButtplugDeviceError::DeviceConnectionError(format!(
      "Error setting up keepalive: {e}"
    )));
  }

  // Spawn the device event forwarding task.
  // This task listens to device events (disconnections, notifications) and forwards them
  // to the device manager event loop via the provided sender.
  let event_stream = device_handle.event_stream();
  let identifier = device_handle.identifier().clone();
  async_manager::spawn(async move {
    futures::pin_mut!(event_stream);
    loop {
      let event = futures::StreamExt::next(&mut event_stream).await;
      match event {
        Some(DeviceEvent::Disconnected(id)) => {
          if device_event_sender
            .send(InternalDeviceEvent::Disconnected(id))
            .await
            .is_err()
          {
            info!(
              "Device event sender closed for device {:?}, stopping event forwarding.",
              identifier
            );
            break;
          }
        }
        Some(DeviceEvent::Notification(id, msg)) => {
          if device_event_sender
            .send(InternalDeviceEvent::Notification(id, msg))
            .await
            .is_err()
          {
            info!(
              "Device event sender closed for device {:?}, stopping event forwarding.",
              identifier
            );
            break;
          }
        }
        None => {
          // Stream ended (device likely disconnected)
          break;
        }
      }
    }
  });

  Ok(device_handle)
}
