// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Representation and management of devices connected to the server.

use crate::ButtplugClientError;

use super::{
  client_device_feature::ClientDeviceFeature,
  create_boxed_future_client_error,
  ButtplugClientMessageSender,
  ButtplugClientResultFuture,
};
use buttplug_core::{
  errors::ButtplugDeviceError,
  message::{
    ButtplugServerMessageV4, DeviceFeature, DeviceFeatureOutput, DeviceMessageInfoV4, FeatureType, OutputCmdV4, OutputCommand, OutputPositionWithDuration, OutputRotateWithDirection, OutputType, OutputValue, StopDeviceCmdV0
  },
  util::stream::convert_broadcast_receiver_to_stream,
};
use futures::{future, FutureExt, Stream};
use getset::{CopyGetters, Getters};
use log::*;
use std::{
  collections::HashMap, fmt, sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  }
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

pub enum ClientDeviceOutputCommand {
  // u32 types use steps, need to compare before sending
  Vibrate(u32),
  Rotate(u32),
  Oscillate(u32),
  Constrict(u32),
  Heater(u32),
  Led(u32),
  Spray(u32),
  Position(u32),
  RotateWithDirection(u32, bool),
  PositionWithDuration(u32, u32),
  // f64 types are old style float, will need to convert before sending
  VibrateFloat(f64),
  RotateFloat(f64),
  OscillateFloat(f64),
  ConstrictFloat(f64),
  HeaterFloat(f64),
  LedFloat(f64),
  SprayFloat(f64),
  PositionFloat(f64),
  RotateWithDirectionFloat(f64, bool),
  PositionWithDurationFloat(f64, u32),
}

impl Into<OutputType> for &ClientDeviceOutputCommand {
  fn into(self) -> OutputType {
    match self {
      ClientDeviceOutputCommand::Vibrate(_) | ClientDeviceOutputCommand::VibrateFloat(_) => OutputType::Vibrate,
      ClientDeviceOutputCommand::Oscillate(_) | ClientDeviceOutputCommand::OscillateFloat(_) => OutputType::Oscillate,
      ClientDeviceOutputCommand::Rotate(_) | ClientDeviceOutputCommand::RotateFloat(_) => OutputType::Rotate,
      ClientDeviceOutputCommand::Constrict(_) | ClientDeviceOutputCommand::ConstrictFloat(_) => OutputType::Constrict,
      ClientDeviceOutputCommand::Heater(_) | ClientDeviceOutputCommand::HeaterFloat(_) => OutputType::Heater,
      ClientDeviceOutputCommand::Led(_) | ClientDeviceOutputCommand::LedFloat(_) => OutputType::Led,
      ClientDeviceOutputCommand::Spray(_) | ClientDeviceOutputCommand::SprayFloat(_) => OutputType::Spray,
      ClientDeviceOutputCommand::Position(_) | ClientDeviceOutputCommand::PositionFloat(_) => OutputType::Position,
      ClientDeviceOutputCommand::PositionWithDuration(_, _) | ClientDeviceOutputCommand::PositionWithDurationFloat(_, _) => OutputType::PositionWithDuration,
      ClientDeviceOutputCommand::RotateWithDirection(_, _) | ClientDeviceOutputCommand::RotateWithDirectionFloat(_, _) => OutputType::RotateWithDirection,
    }
  }
}

impl ClientDeviceOutputCommand {
  fn convert_float_value(&self, feature_output: &DeviceFeatureOutput, float_amt: &f64) -> Result<u32, ButtplugClientError> {
    if *float_amt < 0.0f64 || *float_amt > 1.0f64 {
      Err(ButtplugClientError::ButtplugOutputCommandConversionError("Float values must be between 0.0 and 1.0".to_owned()))
    } else {
      Ok((float_amt * feature_output.step_count() as f64) as u32)
    }
  }

  pub(super) fn to_output_command(&self, feature: &DeviceFeature) -> Result<OutputCommand, ButtplugClientError> {
    for (t, feature_output) in feature.output().as_ref().unwrap_or(&HashMap::new()) {
      if *t == self.into() {
        continue;
      }

      return Ok(match self {
        ClientDeviceOutputCommand::VibrateFloat(v) => OutputCommand::Vibrate(OutputValue::new(self.convert_float_value(feature_output, v)?)),
        ClientDeviceOutputCommand::OscillateFloat(v) => OutputCommand::Oscillate(OutputValue::new(self.convert_float_value(feature_output, v)?)),
        ClientDeviceOutputCommand::RotateFloat(v) => OutputCommand::Rotate(OutputValue::new(self.convert_float_value(feature_output, v)?)),
        ClientDeviceOutputCommand::ConstrictFloat(v) => OutputCommand::Constrict(OutputValue::new(self.convert_float_value(feature_output, v)?)),
        ClientDeviceOutputCommand::HeaterFloat(v) => OutputCommand::Heater(OutputValue::new(self.convert_float_value(feature_output, v)?)),
        ClientDeviceOutputCommand::LedFloat(v )=> OutputCommand::Led(OutputValue::new(self.convert_float_value(feature_output, v)?)),
        ClientDeviceOutputCommand::SprayFloat(v) => OutputCommand::Spray(OutputValue::new(self.convert_float_value(feature_output, v)?)),
        ClientDeviceOutputCommand::PositionFloat(v) => OutputCommand::Position(OutputValue::new(self.convert_float_value(feature_output, v)?)),
        ClientDeviceOutputCommand::PositionWithDurationFloat(v, d) => OutputCommand::PositionWithDuration(OutputPositionWithDuration::new(self.convert_float_value(feature_output, v)?, *d)),
        ClientDeviceOutputCommand::RotateWithDirectionFloat(v, d) => OutputCommand::RotateWithDirection(OutputRotateWithDirection::new(self.convert_float_value(feature_output, v)?, *d)),
        ClientDeviceOutputCommand::Vibrate(v) => OutputCommand::Vibrate(OutputValue::new(*v)),
        ClientDeviceOutputCommand::Oscillate(v) => OutputCommand::Oscillate(OutputValue::new(*v)),
        ClientDeviceOutputCommand::Rotate(v) => OutputCommand::Rotate(OutputValue::new(*v)),
        ClientDeviceOutputCommand::Constrict(v) => OutputCommand::Constrict(OutputValue::new(*v)),
        ClientDeviceOutputCommand::Heater(v) => OutputCommand::Heater(OutputValue::new(*v)),
        ClientDeviceOutputCommand::Led(v )=> OutputCommand::Led(OutputValue::new(*v)),
        ClientDeviceOutputCommand::Spray(v) => OutputCommand::Spray(OutputValue::new(*v)),
        ClientDeviceOutputCommand::Position(v) => OutputCommand::Position(OutputValue::new(*v)),
        ClientDeviceOutputCommand::PositionWithDuration(v, d) => OutputCommand::PositionWithDuration(OutputPositionWithDuration::new(*v, *d)),
        ClientDeviceOutputCommand::RotateWithDirection(v, d) => OutputCommand::RotateWithDirection(OutputRotateWithDirection::new(*v, *d)),
      });
    }
    Err(ButtplugClientError::ButtplugOutputCommandConversionError("Feature does not accomodate type".into()))
  }
}

#[derive(Getters, CopyGetters)]
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
  device_features: HashMap<u32, ClientDeviceFeature>,
  /// Sends commands from the [ButtplugClientDevice] instance to the
  /// [ButtplugClient][super::ButtplugClient]'s event loop, which will then send
  /// the message on to the [ButtplugServer][crate::server::ButtplugServer]
  /// through the connector.
  event_loop_sender: Arc<ButtplugClientMessageSender>,
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
    device_features: &HashMap<u32, DeviceFeature>,
    message_sender: &Arc<ButtplugClientMessageSender>,
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
      device_features: device_features.iter().map(|(i, x)| (*i, ClientDeviceFeature::new(index, *i, &x, &message_sender))).collect(),
      event_loop_sender: message_sender.clone(),
      internal_event_sender: event_sender,
      device_connected,
      client_connected,
    }
  }

  pub(super) fn new_from_device_info(
    info: &DeviceMessageInfoV4,
    sender: &Arc<ButtplugClientMessageSender>,
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

  fn filter_device_actuators(&self, actuator_type: OutputType) -> Vec<ClientDeviceFeature> {
    self
      .device_features
      .iter()
      .filter(|x| {
        x.1
          .feature()
          .output()
          .as_ref()
          .ok_or(false)
          .unwrap()
          .contains_key(&actuator_type)
      })
      .map(|(_, x)| x)
      .cloned()
      .collect()
  }

  fn set_value(&self, output_command: OutputCommand) -> ButtplugClientResultFuture {
    let features = self.filter_device_actuators(output_command.as_output_type());
    if features.is_empty() {
      // TODO err
    }
    let fut_vec: Vec<ButtplugClientResultFuture> = features
      .iter()
      .map(|x| {
        self.event_loop_sender.send_message_expect_ok(
          OutputCmdV4::new(self.index, x.feature_index(), output_command).into(),
        )
      })
      .collect();
    async move {
      futures::future::try_join_all(fut_vec).await?;
      Ok(())
    }
    .boxed()
  }

  fn set_client_value(&self, client_device_command: &ClientDeviceOutputCommand) -> ButtplugClientResultFuture {
    let features = self.filter_device_actuators(client_device_command.into());
    if features.is_empty() {
      // TODO err
    }
    let mut fut_vec: Vec<ButtplugClientResultFuture> = vec!();
    for x in features {
      let val = client_device_command.to_output_command(x.feature());
      match val {
        Ok(v) => fut_vec.push(self.event_loop_sender.send_message_expect_ok(
          OutputCmdV4::new(self.index, x.feature_index(), v).into(),
        )),
        Err(e) => return future::ready(Err(e)).boxed()
     }
    }
    async move {
      futures::future::try_join_all(fut_vec).await?;
      Ok(())
    }
    .boxed()
  }

  pub fn send_command(&self, client_device_command: &ClientDeviceOutputCommand) -> ButtplugClientResultFuture {
    self.set_client_value(client_device_command)
  }

  pub fn vibrate_features(&self) -> Vec<ClientDeviceFeature> {
    self.filter_device_actuators(OutputType::Vibrate)
  }

  /// Commands device to vibrate, assuming it has the features to do so.
  pub fn vibrate(&self, speed: u32) -> ButtplugClientResultFuture {
    self.set_value(OutputCommand::Vibrate(OutputValue::new(speed)))
  }

  pub fn has_battery_level(&self) -> bool {
    self
      .device_features
      .iter()
      .any(|x| x.1.feature().feature_type() == FeatureType::Battery)
  }

  pub fn battery_level(&self) -> ButtplugClientResultFuture<u32> {
    if let Some(battery) = self
      .device_features
      .iter()
      .find(|x| x.1.feature().feature_type() == FeatureType::Battery)
    {
      battery.1.battery_level()
    } else {
      create_boxed_future_client_error(
        ButtplugDeviceError::DeviceFeatureMismatch(
          "Device does not have battery feature available".to_owned(),
        )
        .into(),
      )
    }
  }

  pub fn has_rssi_level(&self) -> bool {
    self
      .device_features
      .iter()
      .any(|x| x.1.feature().feature_type() == FeatureType::Rssi)
  }

  pub fn rssi_level(&self) -> ButtplugClientResultFuture<i8> {
    if let Some(rssi) = self
      .device_features
      .iter()
      .find(|x| x.1.feature().feature_type() == FeatureType::Rssi)
    {
      rssi.1.rssi_level()
    } else {
      create_boxed_future_client_error(
        ButtplugDeviceError::DeviceFeatureMismatch(
          "Device does not have RSSI feature available".to_owned(),
        )
        .into(),
      )
    }
  }

  /// Commands device to stop all movement.
  pub fn stop(&self) -> ButtplugClientResultFuture {
    // All devices accept StopDeviceCmd
    self
      .event_loop_sender
      .send_message_expect_ok(StopDeviceCmdV0::new(self.index).into())
  }

  pub(super) fn set_device_connected(&self, connected: bool) {
    self.device_connected.store(connected, Ordering::Relaxed);
  }

  pub(super) fn set_client_connected(&self, connected: bool) {
    self.client_connected.store(connected, Ordering::Relaxed);
  }

  pub(super) fn queue_event(&self, event: ButtplugClientDeviceEvent) {
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
