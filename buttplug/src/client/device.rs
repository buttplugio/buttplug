// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Representation and management of devices connected to the server.

use super::{ButtplugClientError, ButtplugClientRequest, ButtplugClientResultFuture};
use crate::{
  client::{ButtplugClientMessageFuture, ButtplugClientMessageFuturePair},
  connector::ButtplugConnectorError,
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
    messages::{
      BatteryLevelCmd,      
      ButtplugCurrentSpecClientMessage,
      ButtplugCurrentSpecServerMessage,
      ButtplugDeviceMessageType,
      ButtplugMessage,
      DeviceMessageInfo,
      LinearCmd,
      MessageAttributesMap,
      RotateCmd,
      RotationSubcommand,
      RSSILevelCmd,
      StopDeviceCmd,
      VectorSubcommand,
      VibrateCmd,
      VibrateSubcommand,
    },
  },
  util::async_manager,
};
use async_channel::Sender;
use broadcaster::BroadcastChannel;
use futures::{future, StreamExt};
use std::{
  collections::HashMap,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use tracing_futures::Instrument;

/// Enum for messages going to a [ButtplugClientDevice] instance.
#[derive(Clone)]
pub enum ButtplugClientDeviceEvent {
  /// Device has disconnected from server.
  DeviceDisconnect,
  /// Client has disconnected from server.
  ClientDisconnect,
  /// Message was received from server for that specific device.
  Message(ButtplugCurrentSpecServerMessage),
}

/// Convenience enum for forming [VibrateCmd] commands.
///
/// Allows users to easily specify speeds across different vibration features in
/// a device. Units are in absolute speed values (0.0-1.0).
pub enum VibrateCommand {
  /// Sets all vibration features of a device to the same speed.
  Speed(f64),
  /// Sets vibration features to speed based on the index of the speed in the
  /// vec (i.e. motor 0 is set to `SpeedVec[0]`, motor 1 is set to
  /// `SpeedVec[1]`, etc...)
  SpeedVec(Vec<f64>),
  /// Sets vibration features indicated by index to requested speed. For
  /// instance, if the map has an entry of (1, 0.5), it will set motor 1 to a
  /// speed of 0.5.
  SpeedMap(HashMap<u32, f64>),
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

/// Client-usable representation of device connected to the corresponding
/// [ButtplugServer][crate::server::ButtplugServer]
///
/// [ButtplugClientDevice] instances are obtained from the
/// [ButtplugClient][super::ButtplugClient], and allow the user to send commands
/// to a device connected to the server.
#[derive(Clone)]
pub struct ButtplugClientDevice {
  /// Name of the device
  pub name: String,
  /// Index of the device, matching the index in the
  /// [ButtplugServer][crate::server::ButtplugServer]'s
  /// [DeviceManager][crate::server::device_manager::DeviceManager].
  index: u32,
  /// Map of messages the device can take, along with the attributes of those
  /// messages.
  pub allowed_messages: MessageAttributesMap,
  /// Sends commands from the [ButtplugClientDevice] instance to the
  /// [ButtplugClient][super::ButtplugClient]'s event loop, which will then send
  /// the message on to the [ButtplugServer][crate::server::ButtplugServer]
  /// through the connector.
  message_sender: Sender<ButtplugClientRequest>,
  /// Receives device specific events from the
  /// [ButtplugClient][super::ButtplugClient]'s event loop. Used for device
  /// connection updates, sensor input, etc...
  event_receiver: BroadcastChannel<ButtplugClientDeviceEvent>,
  /// Internal storage for events received from the
  /// [ButtplugClient][super::ButtplugClient].
  // events: Vec<ButtplugClientDeviceEvent>,
  /// True if this [ButtplugClientDevice] is currently connected to the
  /// [ButtplugServer][crate::server::ButtplugServer].
  device_connected: Arc<AtomicBool>,
  /// True if the [ButtplugClient][super::ButtplugClient] that generated this
  /// [ButtplugClientDevice] instance is still connected to the
  /// [ButtplugServer][crate::server::ButtplugServer].
  client_connected: Arc<AtomicBool>,
}

unsafe impl Send for ButtplugClientDevice {
}
unsafe impl Sync for ButtplugClientDevice {
}

impl ButtplugClientDevice {
  /// Creates a new [ButtplugClientDevice] instance
  ///
  /// Fills out the struct members for [ButtplugClientDevice].
  /// `device_connected` and `client_connected` are automatically set to true
  /// because we assume we're only created connected devices.
  ///
  /// # Why is this pub(crate)?
  ///
  /// There's really no reason for anyone but a
  /// [ButtplugClient][super::ButtplugClient] to create a
  /// [ButtplugClientDevice]. A [ButtplugClientDevice] is mostly a shim around
  /// the [ButtplugClient] that generated it, with some added convenience
  /// functions for forming device control messages.
  pub(super) fn new(
    name: &str,
    index: u32,
    allowed_messages: MessageAttributesMap,
    message_sender: Sender<ButtplugClientRequest>,
    event_receiver: BroadcastChannel<ButtplugClientDeviceEvent>,
  ) -> Self {
    info!(
      "Creating client device {} with index {} and messages {:?}.",
      name, index, allowed_messages
    );
    let mut disconnect_receiver = event_receiver.clone();
    let device_connected = Arc::new(AtomicBool::new(true));
    let client_connected = Arc::new(AtomicBool::new(true));
    let device_connected_clone = device_connected.clone();
    let client_connected_clone = client_connected.clone();

    async_manager::spawn(
      async move {
        debug!("Entering client device disconnection loop.");
        loop {
          match disconnect_receiver.recv().await.unwrap() {
            ButtplugClientDeviceEvent::ClientDisconnect => {
              debug!("Client disconnected.");
              device_connected_clone.store(false, Ordering::SeqCst);
              client_connected_clone.store(false, Ordering::SeqCst);
              break;
            }
            ButtplugClientDeviceEvent::DeviceDisconnect => {
              debug!("Device disconnected.");
              device_connected_clone.store(false, Ordering::SeqCst);
              break;
            }
            ButtplugClientDeviceEvent::Message(_) => {
              // To be used once we actually get unrequested info from devices.
              continue;
            }
          }
        }
        debug!("Exiting client device disconnection loop.");
      }
      .instrument(tracing::info_span!(
        "Client Device {} Disconnect Loop",
        index
      )),
    )
    .unwrap();

    Self {
      name: name.to_owned(),
      index,
      allowed_messages,
      message_sender,
      event_receiver,
      device_connected,
      client_connected,
    }
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
    let message_sender = self.message_sender.clone();
    let client_connected = self.client_connected.clone();
    let device_connected = self.device_connected.clone();
    let id = msg.get_id();
    let device_name = self.name.clone();
    Box::pin(
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
        let fut = ButtplugClientMessageFuture::default();
        message_sender
          .send(ButtplugClientRequest::Message(
            ButtplugClientMessageFuturePair::new(msg.clone(), fut.get_state_clone()),
          ))
          .await
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
      .instrument(tracing::trace_span!("ClientDeviceSendFuture for {}", id)),
    )
  }

  pub fn event_receiver(&self) -> impl StreamExt<Item = ButtplugClientDeviceEvent> + Sync + Send {
    self.event_receiver.clone()
  }

  fn create_boxed_future_client_error<T>(&self, err: ButtplugError) -> ButtplugClientResultFuture<T> where T: 'static + Send + Sync {
    Box::pin(future::ready(Err(ButtplugClientError::ButtplugError(err))))
  }

  /// Sends a message, expecting back an [Ok][crate::core::messages::Ok]
  /// message, otherwise returns a [ButtplugError]
  fn send_message_expect_ok(
    &self,
    msg: ButtplugCurrentSpecClientMessage,
  ) -> ButtplugClientResultFuture {
    let send_fut = self.send_message(msg);
    Box::pin(async move {
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
    })
  }

  /// Commands device to vibrate, assuming it has the features to do so.
  pub fn vibrate(&self, speed_cmd: VibrateCommand) -> ButtplugClientResultFuture {
    if !self
      .allowed_messages
      .contains_key(&ButtplugDeviceMessageType::VibrateCmd)
    {
      return self.create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::VibrateCmd).into(),
      );
    }
    let mut vibrator_count: u32 = 0;
    if let Some(features) = self
      .allowed_messages
      .get(&ButtplugDeviceMessageType::VibrateCmd)
    {
      if let Some(v) = features.feature_count {
        vibrator_count = v;
      }
    }
    let mut speed_vec: Vec<VibrateSubcommand>;
    match speed_cmd {
      VibrateCommand::Speed(speed) => {
        speed_vec = Vec::with_capacity(vibrator_count as usize);
        for i in 0..vibrator_count {
          speed_vec.push(VibrateSubcommand::new(i, speed));
        }
      }
      VibrateCommand::SpeedMap(map) => {
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
          speed_vec.push(VibrateSubcommand::new(idx, speed));
        }
      }
      VibrateCommand::SpeedVec(vec) => {
        if vec.len() as u32 > vibrator_count {
          return self.create_boxed_future_client_error(
            ButtplugDeviceError::DeviceFeatureCountMismatch(vibrator_count, vec.len() as u32)
              .into(),
          );
        }
        speed_vec = Vec::with_capacity(vec.len() as usize);
        for (i, v) in vec.iter().enumerate() {
          speed_vec.push(VibrateSubcommand::new(i as u32, *v));
        }
      }
    }
    let msg = VibrateCmd::new(self.index, speed_vec).into();
    self.send_message_expect_ok(msg)
  }

  /// Commands device to move linearly, assuming it has the features to do so.
  pub fn linear(&self, linear_cmd: LinearCommand) -> ButtplugClientResultFuture {
    if !self
      .allowed_messages
      .contains_key(&ButtplugDeviceMessageType::LinearCmd)
    {
      return self.create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::LinearCmd).into(),
      );
    }
    let mut linear_count: u32 = 0;
    if let Some(features) = self
      .allowed_messages
      .get(&ButtplugDeviceMessageType::LinearCmd)
    {
      if let Some(v) = features.feature_count {
        linear_count = v;
      }
    }
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
    if !self
      .allowed_messages
      .contains_key(&ButtplugDeviceMessageType::RotateCmd)
    {
      return self.create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::RotateCmd).into(),
      );
    }
    let mut rotate_count: u32 = 0;
    if let Some(features) = self
      .allowed_messages
      .get(&ButtplugDeviceMessageType::RotateCmd)
    {
      if let Some(v) = features.feature_count {
        rotate_count = v;
      }
    }
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

  pub fn battery_level(&self) -> ButtplugClientResultFuture<f64> {
    if !self
      .allowed_messages
      .contains_key(&ButtplugDeviceMessageType::BatteryLevelCmd)
    {
      return self.create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::BatteryLevelCmd).into(),
      );
    }
    let msg = ButtplugCurrentSpecClientMessage::BatteryLevelCmd(BatteryLevelCmd::new(self.index));
    let send_fut = self.send_message(msg);
    Box::pin(async move {
      match send_fut.await? {
        ButtplugCurrentSpecServerMessage::BatteryLevelReading(reading) => Ok(reading.battery_level),
        ButtplugCurrentSpecServerMessage::Error(err) => Err(ButtplugError::from(err).into()),
        msg => Err(
          ButtplugError::from(ButtplugMessageError::UnexpectedMessageType(format!(
            "{:?}",
            msg
          )))
          .into(),
        ),
      }
    })
  }

  pub fn rssi_level(&self) -> ButtplugClientResultFuture<i32> {
    if !self
      .allowed_messages
      .contains_key(&ButtplugDeviceMessageType::RSSILevelCmd)
    {
      return self.create_boxed_future_client_error(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::RSSILevelCmd).into(),
      );
    }
    let msg = ButtplugCurrentSpecClientMessage::RSSILevelCmd(RSSILevelCmd::new(self.index));
    let send_fut = self.send_message(msg);
    Box::pin(async move {
      match send_fut.await? {
        ButtplugCurrentSpecServerMessage::RSSILevelReading(reading) => Ok(reading.rssi_level),
        ButtplugCurrentSpecServerMessage::Error(err) => Err(ButtplugError::from(err).into()),
        msg => Err(
          ButtplugError::from(ButtplugMessageError::UnexpectedMessageType(format!(
            "{:?}",
            msg
          )))
          .into(),
        ),
      }
    })
  }

  /// Commands device to stop all movement.
  pub fn stop(&self) -> ButtplugClientResultFuture {
    // All devices accept StopDeviceCmd
    self.send_message_expect_ok(StopDeviceCmd::default().into())
  }

  pub fn index(&self) -> u32 {
    self.index
  }
}

impl Eq for ButtplugClientDevice {
}

impl PartialEq for ButtplugClientDevice {
  fn eq(&self, other: &Self) -> bool {
    self.index == other.index
  }
}

impl
  From<(
    &DeviceMessageInfo,
    Sender<ButtplugClientRequest>,
    BroadcastChannel<ButtplugClientDeviceEvent>,
  )> for ButtplugClientDevice
{
  fn from(
    msg_sender_tuple: (
      &DeviceMessageInfo,
      Sender<ButtplugClientRequest>,
      BroadcastChannel<ButtplugClientDeviceEvent>,
    ),
  ) -> Self {
    let msg = msg_sender_tuple.0.clone();
    ButtplugClientDevice::new(
      &*msg.device_name,
      msg.device_index,
      msg.device_messages,
      msg_sender_tuple.1,
      msg_sender_tuple.2,
    )
  }
}
