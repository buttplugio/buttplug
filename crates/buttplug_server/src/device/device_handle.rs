// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Device Handle - Owns device state and handles communication
//!
//! DeviceHandle provides the interface for sending commands to devices.
//! It owns the device state directly and handles all command processing.

use std::{
  collections::BTreeMap,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  time::Duration,
};

use buttplug_core::{
  ButtplugResultFuture,
  errors::{ButtplugDeviceError, ButtplugError},
  message::{
    self, ButtplugMessage, ButtplugServerMessageV4, DeviceFeature, DeviceMessageInfoV4,
    InputCommandType, InputType, OutputValue, StopCmdV4,
  },
  task_span,
  util::async_manager,
  util::stream::convert_broadcast_receiver_to_stream,
  util::task::TaskGroup,
};
use buttplug_server_device_config::{
  DeviceConfigurationManager, ServerDeviceDefinition, ServerDeviceFeatureOutput,
  UserDeviceIdentifier,
};
use dashmap::DashMap;
use futures::future::{self, BoxFuture, FutureExt};
use tokio::{
  select,
  sync::{
    broadcast,
    mpsc::{Sender, channel},
    oneshot,
  },
};
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::{
  ButtplugServerResultFuture,
  message::{
    ButtplugServerDeviceMessage, checked_input_cmd::CheckedInputCmdV4,
    checked_output_cmd::CheckedOutputCmdV4, server_device_attributes::ServerDeviceAttributes,
    spec_enums::ButtplugDeviceCommandMessageUnionV4,
  },
};

use super::{
  InternalDeviceEvent, OutputObservation,
  device_task::{DeviceTaskConfig, DeviceTaskMessage, WRITE_ACK_TIMEOUT, run_owned_device_task},
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
  internal_hw_msg_sender: Sender<DeviceTaskMessage>,
  device_event_sender: Sender<InternalDeviceEvent>,
  disconnect_notified: Arc<AtomicBool>,
  output_observation_sender: Option<broadcast::Sender<OutputObservation>>,
  task_group: TaskGroup,
}

impl DeviceHandle {
  /// Create a new DeviceHandle with direct ownership of device state
  pub(crate) fn new(
    hardware: Arc<Hardware>,
    handler: Arc<dyn ProtocolHandler>,
    definition: ServerDeviceDefinition,
    identifier: UserDeviceIdentifier,
    stop_commands: Vec<ButtplugDeviceCommandMessageUnionV4>,
    internal_hw_msg_sender: Sender<DeviceTaskMessage>,
    device_event_sender: Sender<InternalDeviceEvent>,
    disconnect_notified: Arc<AtomicBool>,
    output_observation_sender: Option<broadcast::Sender<OutputObservation>>,
    task_group: TaskGroup,
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
      device_event_sender,
      disconnect_notified,
      output_observation_sender,
      task_group,
    }
  }

  /// Whether this device needs keepalive packets to maintain its connection.
  ///
  /// Returns true when the protocol handler's keepalive strategy requires periodic
  /// packet replay — either because the hardware requires it (e.g., iOS BLE) or
  /// because the protocol itself specifies timed keepalives.
  pub fn needs_keepalive(&self) -> bool {
    (self.hardware.requires_keepalive()
      && matches!(
        self.handler.keepalive_strategy(),
        ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
      ))
      || matches!(
        self.handler.keepalive_strategy(),
        ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(_)
      )
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
  pub(crate) fn legacy_attributes(&self) -> &ServerDeviceAttributes {
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
        .filter(|(_, f)| f.contains_any_output() || f.contains_any_input())
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
    }
  }

  pub fn stop(&self, stop_cmd: &StopCmdV4) -> ButtplugServerResultFuture {
    // Other generic messages
    self.handle_stop_device_cmd(stop_cmd)
  }

  /// Mark the terminal disconnect notification as already delivered, so neither
  /// the direct disconnect path nor the hardware event forwarding task will send
  /// one. Used when the caller has already removed the device from the manager's
  /// map itself: a queued Disconnected event would be processed after a
  /// replacement device with the same identifier is inserted and remove it.
  pub(super) fn suppress_disconnect_notification(&self) {
    self.disconnect_notified.store(true, Ordering::Release);
  }

  /// Disconnect from the device
  pub fn disconnect(&self) -> ButtplugResultFuture {
    let hardware_disconnect = self.hardware.disconnect();
    let task_group = self.task_group.clone();
    let device_event_sender = self.device_event_sender.clone();
    let disconnect_notified = self.disconnect_notified.clone();
    let identifier = self.identifier.clone();
    async move {
      let hardware_result = hardware_disconnect.await;
      if !disconnect_notified.swap(true, Ordering::AcqRel) {
        let _ = device_event_sender
          .send(InternalDeviceEvent::Disconnected(identifier))
          .await;
      }
      task_group.cancel();
      let _ = task_group.shutdown().await;
      hardware_result.map_err(|err| err.into())
    }
    .boxed()
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

  /// Run an output command through last-command deduplication and observation
  /// emission, returning the protocol handler's hardware commands. Returns None
  /// when the command equals the feature's last command and generates no work.
  /// Shared by the normal output path and the stop path so both keep identical
  /// dedupe-map and observation behaviour.
  fn output_cmd_hardware_commands(
    &self,
    msg: &CheckedOutputCmdV4,
  ) -> Option<Result<Vec<HardwareCommand>, ButtplugError>> {
    if let Some(last_msg) = self.last_output_command.get(&msg.feature_id())
      && *last_msg == *msg
    {
      trace!("No commands generated for incoming device packet, skipping and returning success.");
      return None;
    }
    self
      .last_output_command
      .insert(msg.feature_id(), msg.clone());

    if let Some(sender) = &self.output_observation_sender {
      // OutputType derives Display via strum, producing clean names like "Vibrate", "Rotate".
      // The design uses format!("{:?}") but to_string() is preferred for clean output.
      let _ = sender.send(OutputObservation {
        device_index: self.definition.index(),
        feature_index: msg.feature_index(),
        output_type: msg.output_command().as_output_type().to_string(),
        value: msg.output_command().value() as f64,
      });
    }

    Some(self.handler.handle_output_cmd(msg).map_err(|e| e.into()))
  }

  fn handle_outputcmd_v4(&self, msg: &CheckedOutputCmdV4) -> ButtplugServerResultFuture {
    match self.output_cmd_hardware_commands(msg) {
      None => future::ready(Ok(message::OkV0::default().into())).boxed(),
      Some(Ok(commands)) => self.handle_hardware_commands(commands),
      Some(Err(err)) => future::ready(Err(err)).boxed(),
    }
  }

  fn handle_hardware_commands(&self, commands: Vec<HardwareCommand>) -> ButtplugServerResultFuture {
    let sender = self.internal_hw_msg_sender.clone();
    async move {
      let _ = sender
        .send(DeviceTaskMessage::fire_and_forget(commands))
        .await;
      Ok(message::OkV0::default().into())
    }
    .boxed()
  }

  fn handle_stop_device_cmd(&self, msg: &StopCmdV4) -> ButtplugServerResultFuture {
    let sender = self.internal_hw_msg_sender.clone();
    // Accumulate every per-feature stop OutputCmd into a single
    // write-acknowledged batch so the stop resolves only once the write has
    // reached hardware. Shutdown order is stop-then-disconnect, so without this
    // the disconnect would routinely beat the batched write and drop it.
    let mut hardware_commands: Vec<HardwareCommand> = Vec::new();
    if msg.outputs() {
      for stop_msg in self.stop_commands.iter() {
        if let ButtplugDeviceCommandMessageUnionV4::OutputCmd(checked) = stop_msg
          && let Some(Ok(cmds)) = self.output_cmd_hardware_commands(checked)
        {
          hardware_commands.extend(cmds);
        }
      }
    }
    let input_futs: Vec<_> = if msg.inputs() {
      self
        .definition
        .features()
        .iter()
        .flat_map(|(i, f)| {
          let i = *i;
          let feature_id = f.id();
          f.input.iter().filter_map(move |input| {
            if input.can_subscribe() {
              Some(
                self.parse_message(ButtplugDeviceCommandMessageUnionV4::InputCmd(
                  CheckedInputCmdV4::new(
                    1,
                    self.definition.index(),
                    i,
                    input.input_type(),
                    InputCommandType::Unsubscribe,
                    feature_id,
                  ),
                )),
              )
            } else {
              None
            }
          })
        })
        .collect()
    } else {
      Vec::new()
    };

    async move {
      // Inputs (unsubscribe) are best-effort and do not gate shutdown.
      for fut in input_futs {
        let _ = fut.await;
      }

      if hardware_commands.is_empty() {
        return Ok(message::OkV0::default().into());
      }

      let (message, ack) = DeviceTaskMessage::acknowledged(hardware_commands);
      if sender.send(message).await.is_err() {
        // The device io task is gone (already disconnected). There is nothing to
        // flush, so the stop is satisfied.
        return Ok(message::OkV0::default().into());
      }

      // Bound the wait so a wedged or dead device cannot hang shutdown: a
      // successful ack means the stop write reached hardware; an elapsed timeout
      // still resolves Ok.
      match select! {
        biased;
        result = ack => result,
        _ = async_manager::sleep(WRITE_ACK_TIMEOUT) => Ok(()),
      } {
        Ok(()) => Ok(message::OkV0::default().into()),
        // Receiver dropped without sending: the io task exited mid-flush. The
        // stop may not have landed, but shutdown must not hang on it.
        Err(_) => Ok(message::OkV0::default().into()),
      }
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
  output_observation_sender: Option<broadcast::Sender<OutputObservation>>,
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
  let task_group = TaskGroup::new();
  let (internal_hw_msg_sender, internal_hw_msg_recv) = channel::<DeviceTaskMessage>(1024);

  let device_wait_duration = if let Some(gap) = definition.message_gap_ms() {
    Some(Duration::from_millis(gap as u64))
  } else {
    hardware.message_gap()
  };

  let task_hardware = hardware.clone();
  let task_handler = handler.clone();
  let task_config = DeviceTaskConfig {
    message_gap: device_wait_duration,
    requires_keepalive: hardware.requires_keepalive(),
    keepalive_strategy: handler.keepalive_strategy(),
  };
  task_group
    .spawn(task_span!("DeviceTask"), move || {
      run_owned_device_task(
        task_hardware,
        task_handler,
        task_config,
        internal_hw_msg_recv,
      )
    })
    .map_err(|_| {
      ButtplugDeviceError::DeviceConnectionError(
        "Unable to spawn device task: task group is closed.".to_owned(),
      )
    })?;

  // Generate stop commands for this device
  let mut stop_commands: Vec<ButtplugDeviceCommandMessageUnionV4> = vec![];
  for feature in definition.features().values() {
    for output in feature.output.iter() {
      let mut stop_cmd = |actuator_cmd| {
        stop_commands
          .push(CheckedOutputCmdV4::new(1, 0, feature.index(), feature.id(), actuator_cmd).into());
      };

      // Break out of these if one is found, we only need 1 stop message per output.
      match output {
        ServerDeviceFeatureOutput::Constrict(_) => {
          stop_cmd(message::OutputCommand::Constrict(OutputValue::new(0)));
          break;
        }
        ServerDeviceFeatureOutput::Temperature(_) => {
          stop_cmd(message::OutputCommand::Temperature(OutputValue::new(0)));
          break;
        }
        ServerDeviceFeatureOutput::Spray(_) => {
          stop_cmd(message::OutputCommand::Spray(OutputValue::new(0)));
          break;
        }
        ServerDeviceFeatureOutput::Led(_) => {
          stop_cmd(message::OutputCommand::Led(OutputValue::new(0)));
          break;
        }
        ServerDeviceFeatureOutput::Oscillate(_) => {
          stop_cmd(message::OutputCommand::Oscillate(OutputValue::new(0)));
          break;
        }
        ServerDeviceFeatureOutput::Rotate(_) => {
          stop_cmd(message::OutputCommand::Rotate(OutputValue::new(0)));
          break;
        }
        ServerDeviceFeatureOutput::Vibrate(_) => {
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

  let disconnect_notified = Arc::new(AtomicBool::new(false));

  // Create the DeviceHandle
  let device_handle = DeviceHandle::new(
    hardware,
    handler,
    definition.clone(),
    identifier,
    stop_commands,
    internal_hw_msg_sender,
    device_event_sender.clone(),
    disconnect_notified.clone(),
    output_observation_sender,
    task_group.clone(),
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
    && let Err(e) = device_handle.stop(&StopCmdV4::default()).await
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
  task_group
    .spawn(task_span!("DeviceEventForwarding"), move || async move {
      futures::pin_mut!(event_stream);
      loop {
        let event = futures::StreamExt::next(&mut event_stream).await;
        match event {
          Some(DeviceEvent::Disconnected(id)) => {
            if !disconnect_notified.swap(true, Ordering::AcqRel) {
              if device_event_sender
                .send(InternalDeviceEvent::Disconnected(id))
                .await
                .is_err()
              {
                info!(
                  "Device event sender closed for device {:?}, stopping event forwarding.",
                  identifier
                );
              }
            }
            break;
          }
          Some(DeviceEvent::Notification(_, msg)) => {
            if device_event_sender
              .send(InternalDeviceEvent::Notification(msg))
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
    })
    .map_err(|_| {
      ButtplugDeviceError::DeviceConnectionError(
        "Unable to spawn device event forwarding task: task group is closed.".to_owned(),
      )
    })?;

  Ok(device_handle)
}
