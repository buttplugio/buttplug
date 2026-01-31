// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Representation and management of devices connected to the server.

use crate::ButtplugClientError;
use crate::device::ClientDeviceOutputCommand;

use crate::{
  ButtplugClientMessageSender,
  ButtplugClientResultFuture,
  device::ClientDeviceFeature,
};
use buttplug_core::message::{
  InputType,
  InputTypeReading,
};
use buttplug_core::{
  errors::ButtplugDeviceError,
  message::{ButtplugServerMessageV4, DeviceFeature, DeviceMessageInfoV4, OutputType, StopCmdV4},
  util::stream::convert_broadcast_receiver_to_stream,
};
use futures::{FutureExt, Stream, future};
use getset::{CopyGetters, Getters};
use log::*;
use std::collections::BTreeMap;
use std::{
  fmt,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};
use tokio::sync::broadcast;

/// Enum for messages going to a [ButtplugClientDevice] instance.
#[derive(Clone, Debug)]
// The message enum is what we'll fly with this most of the time. DeviceRemoved/ClientDisconnect
// will happen at most once, so we don't care that those contentless traits still take up > 200
// bytes.
#[allow(clippy::large_enum_variant)]
pub enum ButtplugClientDeviceEvent {
  /// Device has disconnected from server.
  DeviceRemoved,
  /// Client has disconnected from server.
  ClientDisconnect,
  /// Message was received from server for that specific device.
  Message(ButtplugServerMessageV4),
}

#[derive(Getters, CopyGetters, Clone)]
/// Client-usable representation of device connected to the corresponding
/// [ButtplugServer][crate::server::ButtplugServer]
///
/// [ButtplugClientDevice] instances are obtained from the
/// [ButtplugClient][super::ButtplugClient], and allow the user to send commands
/// to a device connected to the server.
pub struct ButtplugClientDevice {
  /// Name of the device
  #[getset(get = "pub")]
  name: String,
  /// Display name of the device
  #[getset(get = "pub")]
  display_name: Option<String>,
  /// Index of the device, matching the index in the
  /// [ButtplugServer][crate::server::ButtplugServer]'s
  /// [DeviceManager][crate::server::device_manager::DeviceManager].
  #[getset(get_copy = "pub")]
  index: u32,
  /// Actuators and sensors available on the device.
  #[getset(get = "pub")]
  device_features: BTreeMap<u32, ClientDeviceFeature>,
  /// Sends commands from the [ButtplugClientDevice] instance to the
  /// [ButtplugClient][super::ButtplugClient]'s event loop, which will then send
  /// the message on to the [ButtplugServer][crate::server::ButtplugServer]
  /// through the connector.
  event_loop_sender: ButtplugClientMessageSender,
  internal_event_sender: broadcast::Sender<ButtplugClientDeviceEvent>,
  /// True if this [ButtplugClientDevice] is currently connected to the
  /// [ButtplugServer][crate::server::ButtplugServer].
  device_connected: Arc<AtomicBool>,
  /// True if the [ButtplugClient][super::ButtplugClient] that generated this
  /// [ButtplugClientDevice] instance is still connected to the
  /// [ButtplugServer][crate::server::ButtplugServer].
  client_connected: Arc<AtomicBool>,
}

impl ButtplugClientDevice {
  /// Creates a new [ButtplugClientDevice] instance
  ///
  /// Fills out the struct members for [ButtplugClientDevice].
  /// `device_connected` and `client_connected` are automatically set to true
  /// because we assume we're only created connected devices.
  ///
  /// # Why is this pub(super)?
  ///
  /// There's really no reason for anyone but a
  /// [ButtplugClient][super::ButtplugClient] to create a
  /// [ButtplugClientDevice]. A [ButtplugClientDevice] is mostly a shim around
  /// the [ButtplugClient] that generated it, with some added convenience
  /// functions for forming device control messages.
  pub(super) fn new(
    name: &str,
    display_name: &Option<String>,
    index: u32,
    device_features: &BTreeMap<u32, DeviceFeature>,
    message_sender: &ButtplugClientMessageSender,
  ) -> Self {
    info!(
      "Creating client device {} with index {} and messages {:?}.",
      name, index, device_features
    );
    let (event_sender, _) = broadcast::channel(256);
    let device_connected = Arc::new(AtomicBool::new(true));
    let client_connected = Arc::new(AtomicBool::new(true));

    Self {
      name: name.to_owned(),
      display_name: display_name.clone(),
      index,
      device_features: device_features
        .iter()
        .map(|(i, x)| (*i, ClientDeviceFeature::new(index, *i, x, message_sender)))
        .collect(),
      event_loop_sender: message_sender.clone(),
      internal_event_sender: event_sender,
      device_connected,
      client_connected,
    }
  }

  pub(crate) fn new_from_device_info(
    info: &DeviceMessageInfoV4,
    sender: &ButtplugClientMessageSender,
  ) -> Self {
    ButtplugClientDevice::new(
      info.device_name(),
      info.device_display_name(),
      info.device_index(),
      info.device_features(),
      sender,
    )
  }

  pub fn connected(&self) -> bool {
    self.device_connected.load(Ordering::Relaxed)
  }

  pub fn event_stream(&self) -> Box<dyn Stream<Item = ButtplugClientDeviceEvent> + Send + Unpin> {
    Box::new(Box::pin(convert_broadcast_receiver_to_stream(
      self.internal_event_sender.subscribe(),
    )))
  }

  fn filter_device_outputs(&self, actuator_type: OutputType) -> Vec<ClientDeviceFeature> {
    self
      .device_features
      .iter()
      .filter(|x| {
        if let Some(output) = x.1.feature().output() {
          output.contains(actuator_type)
        } else {
          false
        }
      })
      .map(|(_, x)| x)
      .cloned()
      .collect()
  }

  fn set_client_value(
    &self,
    client_device_command: &ClientDeviceOutputCommand,
  ) -> ButtplugClientResultFuture {
    let features = self.filter_device_outputs(client_device_command.into());
    if features.is_empty() {
      // TODO err
    }
    let mut fut_vec: Vec<ButtplugClientResultFuture> = vec![];
    for x in features {
      let val = x.convert_client_cmd_to_output_cmd(client_device_command);
      match val {
        Ok(v) => fut_vec.push(self.event_loop_sender.send_message_expect_ok(v.into())),
        Err(e) => return future::ready(Err(e)).boxed(),
      }
    }
    async move {
      futures::future::try_join_all(fut_vec).await?;
      Ok(())
    }
    .boxed()
  }

  pub fn outputs(&self, output_type: OutputType) -> Vec<ClientDeviceFeature> {
    self.filter_device_outputs(output_type)
  }

  pub fn output_available(&self, output_type: OutputType) -> bool {
    !self.filter_device_outputs(output_type).is_empty()
  }

  pub fn run_output(
    &self,
    client_device_command: &ClientDeviceOutputCommand,
  ) -> ButtplugClientResultFuture {
    self.set_client_value(client_device_command)
  }

  pub fn input_available(&self, input_type: InputType) -> bool {
    self.device_features.iter().any(|x| {
      x.1
        .feature()
        .input()
        .as_ref()
        .is_some_and(|x| x.contains(input_type))
    })
  }

  fn input_feature(
    &self,
    input_type: InputType,
  ) -> Result<&ClientDeviceFeature, ButtplugClientError> {
    let inputs: Vec<_> = self
      .device_features
      .iter()
      .filter(|x| {
        x.1
          .feature()
          .input()
          .as_ref()
          .is_some_and(|x| x.contains(input_type))
      })
      .collect();
    let input_count = inputs.len();
    if input_count > 1 {
      Err(ButtplugClientError::ButtplugMultipleInputAvailableError(
        input_type,
      ))
    } else if input_count == 0 {
      Err(ButtplugClientError::ButtplugError(
        ButtplugDeviceError::DeviceNoInputError(input_type).into(),
      ))
    } else {
      Ok(inputs[0].1)
    }
  }

  pub fn run_input_read(&self, input_type: InputType) -> ButtplugClientResultFuture<InputTypeReading> {
    match self.input_feature(input_type) {
      Ok(dev) => dev.run_input_read(input_type).boxed(),
      Err(e) => future::ready(Err(e)).boxed(),
    }
  }

  pub fn run_input_subscribe(&self, input_type: InputType) -> ButtplugClientResultFuture {
    match self.input_feature(input_type) {
      Ok(dev) => dev.run_input_subscribe(input_type).boxed(),
      Err(e) => future::ready(Err(e)).boxed(),
    }
  }

  pub fn run_input_unsubscribe(&self, input_type: InputType) -> ButtplugClientResultFuture {
    match self.input_feature(input_type) {
      Ok(dev) => dev.run_input_unsubscribe(input_type).boxed(),
      Err(e) => future::ready(Err(e)).boxed(),
    }
  }

  pub fn battery(&self) -> ButtplugClientResultFuture<u32> {
    let fut = self.run_input_read(InputType::Battery);
    async move {
      let val = fut.await?;
      if let InputTypeReading::Battery(x) = val {
        Ok(x.data().into())
      } else {
        Err(ButtplugClientError::ButtplugError(
          ButtplugDeviceError::DeviceNoInputError(val.into()).into(),
        ))
      }
    }
    .boxed()
  }

  pub fn rssi(&self) -> ButtplugClientResultFuture<i8> {
    let fut = self.run_input_read(InputType::Rssi);
    async move {
      let val = fut.await?;
      if let InputTypeReading::Rssi(x) = val {
        Ok(x.data().into())
      } else {
        Err(ButtplugClientError::ButtplugError(
          ButtplugDeviceError::DeviceNoInputError(val.into()).into(),
        ))
      }
    }
    .boxed()
  }

  /// Commands device to stop all movement.
  pub fn stop(&self) -> ButtplugClientResultFuture {
    // All devices accept StopDeviceCmd
    self
      .event_loop_sender
      .send_message_expect_ok(StopCmdV4::new(Some(self.index), None, true, true).into())
  }

  pub fn stop_features(&self, inputs: bool, outputs: bool) -> ButtplugClientResultFuture {
    // All devices accept StopDeviceCmd
    self
      .event_loop_sender
      .send_message_expect_ok(StopCmdV4::new(Some(self.index), None, inputs, outputs).into())
  }

  pub(crate) fn set_device_connected(&self, connected: bool) {
    self.device_connected.store(connected, Ordering::Relaxed);
  }

  pub(crate) fn set_client_connected(&self, connected: bool) {
    self.client_connected.store(connected, Ordering::Relaxed);
  }

  pub(crate) fn queue_event(&self, event: ButtplugClientDeviceEvent) {
    if self.internal_event_sender.receiver_count() == 0 {
      // We can drop devices before we've hooked up listeners or after the device manager drops,
      // which is common, so only show this when in debug.
      debug!("No handlers for device event, dropping event: {:?}", event);
      return;
    }
    self
      .internal_event_sender
      .send(event)
      .expect("Checked for receivers already.");
  }
}

impl Eq for ButtplugClientDevice {
}

impl PartialEq for ButtplugClientDevice {
  fn eq(&self, other: &Self) -> bool {
    self.index == other.index
  }
}

impl fmt::Debug for ButtplugClientDevice {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ButtplugClientDevice")
      .field("name", &self.name)
      .field("index", &self.index)
      .finish()
  }
}
