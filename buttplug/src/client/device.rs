// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Representation and management of devices connected to the server.

use super::{
  ButtplugClientError,
  ButtplugClientMessageFuturePair,
  ButtplugClientRequest,
  ButtplugClientResultFuture,
  ButtplugServerMessageFuture,
};
use crate::{
  core::{
    connector::ButtplugConnectorError,
    errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
    messages::{
      ActuatorType,
      ButtplugCurrentSpecClientMessage,
      ButtplugCurrentSpecDeviceMessageType,
      ButtplugCurrentSpecServerMessage,
      ButtplugDeviceMessageType,
      ButtplugMessage,
      DeviceMessageAttributes,
      DeviceMessageInfo,
      Endpoint,
      LinearCmd,
      RawReadCmd,
      RawSubscribeCmd,
      RawUnsubscribeCmd,
      RawWriteCmd,
      RotateCmd,
      RotationSubcommand,
      ScalarCmd,
      ScalarSubcommand,
      SensorReadCmd,
      SensorSubscribeCmd,
      SensorType,
      SensorUnsubscribeCmd,
      StopDeviceCmd,
      VectorSubcommand,
    },
  },
  util::stream::convert_broadcast_receiver_to_stream,
};
use futures::{future, FutureExt, Stream};
use std::{
  collections::HashMap,
  fmt,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use tokio::sync::broadcast;
use tracing_futures::Instrument;

/// Enum for messages going to a [ButtplugClientDevice] instance.
#[derive(Clone, Debug)]
pub enum ButtplugClientDeviceEvent {
  /// Device has disconnected from server.
  DeviceRemoved,
  /// Client has disconnected from server.
  ClientDisconnect,
  /// Message was received from server for that specific device.
  Message(ButtplugCurrentSpecServerMessage),
}

/// Convenience enum for forming [VibrateCmd] commands.
///
/// Allows users to easily specify speeds across different vibration features in
/// a device. Units are in absolute speed values (0.0-1.0).
pub enum ScalarCommand {
  /// Sets all vibration features of a device to the same speed.
  Scalar(f64),
  /// Sets vibration features to speed based on the index of the speed in the
  /// vec (i.e. motor 0 is set to `SpeedVec[0]`, motor 1 is set to
  /// `SpeedVec[1]`, etc...)
  ScalarVec(Vec<f64>),
  /// Sets vibration features indicated by index to requested speed. For
  /// instance, if the map has an entry of (1, 0.5), it will set motor 1 to a
  /// speed of 0.5.
  ScalarMap(HashMap<u32, f64>),
}

/// Convenience enum for forming [RotateCmd] commands.
///
/// Allows users to easily specify speeds/directions across different rotation
/// features in a device. Units are in absolute speed (0.0-1.0), and clockwise
/// direction (clockwise if true, counterclockwise if false)
pub enum RotateCommand {
  /// Sets all rotation features of a device to the same speed/direction.
  Rotate(f64, bool),
  /// Sets rotation features to speed/direction based on the index of the
  /// speed/rotation pair in the vec (i.e. motor 0 speed/direction is set to
  /// `RotateVec[0]`, motor 1 is set to `RotateVec[1]`, etc...)
  RotateVec(Vec<(f64, bool)>),
  /// Sets rotation features indicated by index to requested speed/direction.
  /// For instance, if the map has an entry of (1, (0.5, true)), it will set
  /// motor 1 to rotate at a speed of 0.5, in the clockwise direction.
  RotateMap(HashMap<u32, (f64, bool)>),
}

/// Convenience enum for forming [LinearCmd] commands.
///
/// Allows users to easily specify position/durations across different rotation
/// features in a device. Units are in absolute position (0.0-1.0) and
/// millliseconds of movement duration.
pub enum LinearCommand {
  /// Sets all linear features of a device to the same position/duration.
  Linear(u32, f64),
  /// Sets linear features to position/duration based on the index of the
  /// position/duration pair in the vec (i.e. motor 0 position/duration is set to
  /// `LinearVec[0]`, motor 1 is set to `LinearVec[1]`, etc...)
  LinearVec(Vec<(u32, f64)>),
  /// Sets linear features indicated by index to requested position/duration.
  /// For instance, if the map has an entry of (1, (0.5, 500)), it will set
  /// motor 1 to move to position 0.5 over the course of 500ms.
  LinearMap(HashMap<u32, (u32, f64)>),
}

pub type ButtplugClientDeviceMessageType = ButtplugCurrentSpecDeviceMessageType;

/// Client-usable representation of device connected to the corresponding
/// [ButtplugServer][crate::server::ButtplugServer]
///
/// [ButtplugClientDevice] instances are obtained from the
/// [ButtplugClient][super::ButtplugClient], and allow the user to send commands
/// to a device connected to the server.
pub struct ButtplugClientDevice {
  /// Name of the device
  pub name: String,
  /// Index of the device, matching the index in the
  /// [ButtplugServer][crate::server::ButtplugServer]'s
  /// [DeviceManager][crate::server::device_manager::DeviceManager].
  index: u32,
  /// Map of messages the device can take, along with the attributes of those
  /// messages.
  pub message_attributes: DeviceMessageAttributes,
  /// Sends commands from the [ButtplugClientDevice] instance to the
  /// [ButtplugClient][super::ButtplugClient]'s event loop, which will then send
  /// the message on to the [ButtplugServer][crate::server::ButtplugServer]
  /// through the connector.
  event_loop_sender: broadcast::Sender<ButtplugClientRequest>,
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
    index: u32,
    message_attributes: &DeviceMessageAttributes,
    message_sender: broadcast::Sender<ButtplugClientRequest>,
  ) -> Self {
    info!(
      "Creating client device {} with index {} and messages {:?}.",
      name, index, message_attributes
    );
    let (event_sender, _) = broadcast::channel(256);
    let device_connected = Arc::new(AtomicBool::new(true));
    let client_connected = Arc::new(AtomicBool::new(true));

    Self {
      name: name.to_owned(),
      index,
      message_attributes: message_attributes.clone(),
      event_loop_sender: message_sender,
      internal_event_sender: event_sender,
      device_connected,
      client_connected,
    }
  }

  pub(super) fn new_from_device_info(
    info: &DeviceMessageInfo,
    sender: broadcast::Sender<ButtplugClientRequest>,
  ) -> Self {
    ButtplugClientDevice::new(
      &*info.device_name,
      info.device_index,
      &info.device_messages,
      sender,
    )
  }

  pub fn connected(&self) -> bool {
    self.device_connected.load(Ordering::SeqCst)
  }

  /// Sends a message through the owning
  /// [ButtplugClient][super::ButtplugClient].
  ///
  /// Performs the send/receive flow for send a device command and receiving the
  /// response from the server.
  fn send_message(
    &self,
    msg: ButtplugCurrentSpecClientMessage,
  ) -> ButtplugClientResultFuture<ButtplugCurrentSpecServerMessage> {
    let message_sender = self.event_loop_sender.clone();
    let client_connected = self.client_connected.clone();
    let device_connected = self.device_connected.clone();
    let id = msg.id();
    let device_name = self.name.clone();
    async move {
      if !client_connected.load(Ordering::SeqCst) {
        error!("Client not connected, cannot run device command");
        return Err(ButtplugConnectorError::ConnectorNotConnected.into());
      } else if !device_connected.load(Ordering::SeqCst) {
        error!("Device not connected, cannot run device command");
        return Err(
          ButtplugError::from(ButtplugDeviceError::DeviceNotConnected(device_name)).into(),
        );
      }
      let fut = ButtplugServerMessageFuture::default();
      message_sender
        .send(ButtplugClientRequest::Message(
          ButtplugClientMessageFuturePair::new(msg.clone(), fut.get_state_clone()),
        ))
        .map_err(|_| {
          ButtplugClientError::ButtplugConnectorError(
            ButtplugConnectorError::ConnectorChannelClosed,
          )
        })?;
      let msg = fut.await?;
      if let ButtplugCurrentSpecServerMessage::Error(_err) = msg {
        Err(ButtplugError::from(_err).into())
      } else {
        Ok(msg)
      }
    }
    .instrument(tracing::trace_span!("ClientDeviceSendFuture for {}", id))
    .boxed()
  }

  pub fn event_stream(&self) -> Box<dyn Stream<Item = ButtplugClientDeviceEvent> + Send + Unpin> {
    Box::new(Box::pin(convert_broadcast_receiver_to_stream(
      self.internal_event_sender.subscribe(),
    )))
  }

  fn create_boxed_future_client_error<T>(&self, err: ButtplugError) -> ButtplugClientResultFuture<T>
  where
    T: 'static + Send + Sync,
  {
    future::ready(Err(ButtplugClientError::ButtplugError(err))).boxed()
  }

  /// Sends a message, expecting back an [Ok][crate::core::messages::Ok]
  /// message, otherwise returns a [ButtplugError]
  fn send_message_expect_ok(
    &self,
    msg: ButtplugCurrentSpecClientMessage,
  ) -> ButtplugClientResultFuture {
    let send_fut = self.send_message(msg);
    async move {
      match send_fut.await? {
        ButtplugCurrentSpecServerMessage::Ok(_) => Ok(()),
        ButtplugCurrentSpecServerMessage::Error(_err) => Err(ButtplugError::from(_err).into()),
        msg => Err(
          ButtplugError::from(ButtplugMessageError::UnexpectedMessageType(format!(
            "{:?}",
            msg
          )))
          .into(),
        ),
      }
    }.boxed()
  }

  /// Commands device to vibrate, assuming it has the features to do so.
  pub fn vibrate(&self, speed_cmd: ScalarCommand) -> ButtplugClientResultFuture {
    if self.message_attributes.scalar_cmd().is_none() {
      return self.create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::VibrateCmd).into(),
      );
    }

    let vibrator_count: u32 = self
      .message_attributes
      .scalar_cmd()
      .as_ref()
      .expect("Already checked existence")
      .iter()
      .filter(|x| *x.actuator_type() == ActuatorType::Vibrate)
      .count() as u32;

    let mut speed_vec: Vec<ScalarSubcommand>;
    match speed_cmd {
      ScalarCommand::Scalar(speed) => {
        speed_vec = Vec::with_capacity(vibrator_count as usize);
        for i in 0..vibrator_count {
          speed_vec.push(ScalarSubcommand::new(i, speed, ActuatorType::Vibrate));
        }
      }
      ScalarCommand::ScalarMap(map) => {
        if map.len() as u32 > vibrator_count {
          return self.create_boxed_future_client_error(
            ButtplugDeviceError::DeviceFeatureCountMismatch(vibrator_count, map.len() as u32)
              .into(),
          );
        }
        speed_vec = Vec::with_capacity(map.len() as usize);
        for (idx, speed) in map {
          if idx > vibrator_count - 1 {
            return self.create_boxed_future_client_error(
              ButtplugDeviceError::DeviceFeatureIndexError(vibrator_count, idx).into(),
            );
          }
          speed_vec.push(ScalarSubcommand::new(idx, speed, ActuatorType::Vibrate));
        }
      }
      ScalarCommand::ScalarVec(vec) => {
        if vec.len() as u32 > vibrator_count {
          return self.create_boxed_future_client_error(
            ButtplugDeviceError::DeviceFeatureCountMismatch(vibrator_count, vec.len() as u32)
              .into(),
          );
        }
        speed_vec = Vec::with_capacity(vec.len() as usize);
        for (i, v) in vec.iter().enumerate() {
          speed_vec.push(ScalarSubcommand::new(i as u32, *v, ActuatorType::Vibrate));
        }
      }
    }
    let msg = ScalarCmd::new(self.index, speed_vec).into();
    self.send_message_expect_ok(msg)
  }

  /// Commands device to move linearly, assuming it has the features to do so.
  pub fn linear(&self, linear_cmd: LinearCommand) -> ButtplugClientResultFuture {
    if self.message_attributes.linear_cmd().is_none() {
      return self.create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::LinearCmd).into(),
      );
    }

    let linear_count: u32 = self.message_attributes.linear_cmd().as_ref().unwrap().len() as u32;

    let mut linear_vec: Vec<VectorSubcommand>;
    match linear_cmd {
      LinearCommand::Linear(dur, pos) => {
        linear_vec = Vec::with_capacity(linear_count as usize);
        for i in 0..linear_count {
          linear_vec.push(VectorSubcommand::new(i, dur, pos));
        }
      }
      LinearCommand::LinearMap(map) => {
        if map.len() as u32 > linear_count {
          return self.create_boxed_future_client_error(
            ButtplugDeviceError::DeviceFeatureCountMismatch(linear_count, map.len() as u32).into(),
          );
        }
        linear_vec = Vec::with_capacity(map.len() as usize);
        for (idx, (dur, pos)) in map {
          if idx > linear_count - 1 {
            return self.create_boxed_future_client_error(
              ButtplugDeviceError::DeviceFeatureIndexError(linear_count, idx).into(),
            );
          }
          linear_vec.push(VectorSubcommand::new(idx, dur, pos));
        }
      }
      LinearCommand::LinearVec(vec) => {
        if vec.len() as u32 > linear_count {
          return self.create_boxed_future_client_error(
            ButtplugDeviceError::DeviceFeatureCountMismatch(linear_count, vec.len() as u32).into(),
          );
        }
        linear_vec = Vec::with_capacity(vec.len() as usize);
        for (i, v) in vec.iter().enumerate() {
          linear_vec.push(VectorSubcommand::new(i as u32, v.0, v.1));
        }
      }
    }
    let msg = LinearCmd::new(self.index, linear_vec).into();
    self.send_message_expect_ok(msg)
  }

  /// Commands device to rotate, assuming it has the features to do so.
  pub fn rotate(&self, rotate_cmd: RotateCommand) -> ButtplugClientResultFuture {
    if self.message_attributes.rotate_cmd().is_none() {
      return self.create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::RotateCmd).into(),
      );
    }

    let rotate_count: u32 = self.message_attributes.rotate_cmd().as_ref().unwrap().len() as u32;

    let mut rotate_vec: Vec<RotationSubcommand>;
    match rotate_cmd {
      RotateCommand::Rotate(speed, clockwise) => {
        rotate_vec = Vec::with_capacity(rotate_count as usize);
        for i in 0..rotate_count {
          rotate_vec.push(RotationSubcommand::new(i, speed, clockwise));
        }
      }
      RotateCommand::RotateMap(map) => {
        if map.len() as u32 > rotate_count {
          return self.create_boxed_future_client_error(
            ButtplugDeviceError::DeviceFeatureCountMismatch(rotate_count, map.len() as u32).into(),
          );
        }
        rotate_vec = Vec::with_capacity(map.len() as usize);
        for (idx, (speed, clockwise)) in map {
          if idx > rotate_count - 1 {
            return self.create_boxed_future_client_error(
              ButtplugDeviceError::DeviceFeatureIndexError(rotate_count, idx).into(),
            );
          }
          rotate_vec.push(RotationSubcommand::new(idx, speed, clockwise));
        }
      }
      RotateCommand::RotateVec(vec) => {
        if vec.len() as u32 > rotate_count {
          return self.create_boxed_future_client_error(
            ButtplugDeviceError::DeviceFeatureCountMismatch(rotate_count, vec.len() as u32).into(),
          );
        }
        rotate_vec = Vec::with_capacity(vec.len() as usize);
        for (i, v) in vec.iter().enumerate() {
          rotate_vec.push(RotationSubcommand::new(i as u32, v.0, v.1));
        }
      }
    }
    let msg = RotateCmd::new(self.index, rotate_vec).into();
    self.send_message_expect_ok(msg)
  }

  pub fn subscribe_pressure(&self) -> ButtplugClientResultFuture {
    if self.message_attributes.sensor_subscribe_cmd().is_none() {
      return self.create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::SensorSubscribeCmd)
          .into(),
      );
    }
    let msg = SensorSubscribeCmd::new(self.index, 0, SensorType::Pressure).into();
    self.send_message_expect_ok(msg)
  }

  pub fn unsubscribe_pressure(&self) -> ButtplugClientResultFuture {
    if self.message_attributes.sensor_subscribe_cmd().is_none() {
      return self.create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::SensorSubscribeCmd)
          .into(),
      );
    }
    let msg = SensorUnsubscribeCmd::new(self.index, 0, SensorType::Pressure).into();
    self.send_message_expect_ok(msg)
  }

  fn read_single_sensor(&self, sensor_type: &SensorType) -> ButtplugClientResultFuture<Vec<i32>> {
    if self.message_attributes.sensor_read_cmd().is_none() {
      return self.create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::SensorReadCmd).into(),
      );
    }
    let sensor_indexes: Vec<u32> = self
      .message_attributes
      .sensor_read_cmd()
      .as_ref()
      .expect("Already check existence")
      .iter()
      .enumerate()
      .filter(|x| *x.1.sensor_type() == *sensor_type)
      .map(|x| x.0 as u32)
      .collect();
    if sensor_indexes.len() != 1 {
      return self.create_boxed_future_client_error(
        ButtplugDeviceError::ProtocolSensorNotSupported(*sensor_type).into(),
      );
    }    
    let msg = SensorReadCmd::new(self.index, sensor_indexes[0], *sensor_type).into();
    let reply = self.send_message(msg);
    async move {    
      if let ButtplugCurrentSpecServerMessage::SensorReading(data) = reply.await? {
        Ok(data.data().clone())
      } else {
        Err(ButtplugError::ButtplugMessageError(ButtplugMessageError::UnexpectedMessageType("SensorReading".to_owned())).into())
      }
    }.boxed()
  }

  pub fn battery_level(&self) -> ButtplugClientResultFuture<f64> {
    let send_fut = self.read_single_sensor(&SensorType::Battery);
    Box::pin(async move {
      let data = send_fut.await?;
      let battery_level = data[0];
      Ok(battery_level as f64 / 100.0f64)
    })
  }

  pub fn rssi_level(&self) -> ButtplugClientResultFuture<i32> {
    let send_fut = self.read_single_sensor(&SensorType::RSSI);
    Box::pin(async move {
      let data = send_fut.await?;
      Ok(data[0])
    })
  }

  pub fn raw_write(
    &self,
    endpoint: Endpoint,
    data: Vec<u8>,
    write_with_response: bool,
  ) -> ButtplugClientResultFuture {
    if self.message_attributes.raw_write_cmd().is_none() {
      return self.create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::RawWriteCmd).into(),
      );
    }
    let msg = ButtplugCurrentSpecClientMessage::RawWriteCmd(RawWriteCmd::new(
      self.index,
      endpoint,
      data,
      write_with_response,
    ));
    self.send_message_expect_ok(msg)
  }

  pub fn raw_read(
    &self,
    endpoint: Endpoint,
    expected_length: u32,
    timeout: u32,
  ) -> ButtplugClientResultFuture<Vec<u8>> {
    if self.message_attributes.raw_read_cmd().is_none() {
      return self.create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::RawReadCmd).into(),
      );
    }
    let msg = ButtplugCurrentSpecClientMessage::RawReadCmd(RawReadCmd::new(
      self.index,
      endpoint,
      expected_length,
      timeout,
    ));
    let send_fut = self.send_message(msg);
    async move {
      match send_fut.await? {
        ButtplugCurrentSpecServerMessage::RawReading(reading) => Ok(reading.data().clone()),
        ButtplugCurrentSpecServerMessage::Error(err) => Err(ButtplugError::from(err).into()),
        msg => Err(
          ButtplugError::from(ButtplugMessageError::UnexpectedMessageType(format!(
            "{:?}",
            msg
          )))
          .into(),
        ),
      }
    }.boxed()
  }

  pub fn raw_subscribe(&self, endpoint: Endpoint) -> ButtplugClientResultFuture {
    if self.message_attributes.raw_subscribe_cmd().is_none() {
      return self.create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::RawSubscribeCmd).into(),
      );
    }
    let msg =
      ButtplugCurrentSpecClientMessage::RawSubscribeCmd(RawSubscribeCmd::new(self.index, endpoint));
    self.send_message_expect_ok(msg)
  }

  pub fn raw_unsubscribe(&self, endpoint: Endpoint) -> ButtplugClientResultFuture {
    if self.message_attributes.raw_subscribe_cmd().is_none() {
      return self.create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::RawSubscribeCmd).into(),
      );
    }
    let msg = ButtplugCurrentSpecClientMessage::RawUnsubscribeCmd(RawUnsubscribeCmd::new(
      self.index, endpoint,
    ));
    self.send_message_expect_ok(msg)
  }

  /// Commands device to stop all movement.
  pub fn stop(&self) -> ButtplugClientResultFuture {
    // All devices accept StopDeviceCmd
    self.send_message_expect_ok(StopDeviceCmd::new(self.index).into())
  }

  pub fn index(&self) -> u32 {
    self.index
  }

  pub(super) fn set_device_connected(&self, connected: bool) {
    self.device_connected.store(connected, Ordering::SeqCst);
  }

  pub(super) fn set_client_connected(&self, connected: bool) {
    self.client_connected.store(connected, Ordering::SeqCst);
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
