// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.
//! Representation and management of devices connected to the server.

use super::{
  client::{
    ButtplugClientError, ButtplugClientMessageFuturePair, ButtplugClientResultFuture,
    ButtplugServerMessageSender,
  },
  client_event_loop::ButtplugClientRequest,
};
use buttplug_core::{
  connector::ButtplugConnectorError,
  errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
  message::ButtplugMessage,
  util::stream::convert_broadcast_receiver_to_stream,
};
use buttplug_server::message::{
  ButtplugClientMessageV0, ButtplugDeviceMessageNameV0, ButtplugServerMessageV0,
  SingleMotorVibrateCmdV0, StopDeviceCmdV0,
};
use futures::channel::oneshot;
use futures::{Stream, future};
use getset::Getters;
use log::*;
use std::{
  fmt,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};
use tokio::sync::{broadcast, mpsc};
use tracing::Instrument;

/// Enum for messages going to a [ButtplugClientDevice] instance.
#[derive(Clone, Debug)]
pub enum ButtplugClientDeviceEvent {
  /// Device has disconnected from server.
  DeviceRemoved,
  /// Client has disconnected from server.
  ClientDisconnect,
  /// Message was received from server for that specific device.
  Message(ButtplugServerMessageV0),
}

/// Client-usable representation of device connected to the corresponding
/// [ButtplugServer][crate::server::ButtplugServer]
///
/// [ButtplugClientDevice] instances are obtained from the
/// [ButtplugClient][super::ButtplugClient], and allow the user to send commands
/// to a device connected to the server.
///
/// V0 devices are significantly simpler than V2+ devices because V0 only supports
/// SingleMotorVibrateCmd (single speed for all motors) and StopDeviceCmd.
#[derive(Getters)]
pub struct ButtplugClientDevice {
  /// Name of the device
  #[getset(get = "pub")]
  name: String,
  /// Index of the device, matching the index in the
  /// [ButtplugServer][crate::server::ButtplugServer]'s
  /// [DeviceManager][crate::server::device_manager::DeviceManager].
  index: u32,
  /// Flat list of message names the device supports (V0 uses message names, not structured attributes)
  #[getset(get = "pub")]
  device_messages: Vec<ButtplugDeviceMessageNameV0>,
  /// Sends commands from the [ButtplugClientDevice] instance to the
  /// [ButtplugClient][super::ButtplugClient]'s event loop, which will then send
  /// the message on to the [ButtplugServer][crate::server::ButtplugServer]
  /// through the connector.
  event_loop_sender: mpsc::Sender<ButtplugClientRequest>,
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
    device_messages: Vec<ButtplugDeviceMessageNameV0>,
    message_sender: mpsc::Sender<ButtplugClientRequest>,
  ) -> Self {
    info!(
      "Creating client device {} with index {} and messages {:?}.",
      name, index, device_messages
    );
    let (event_sender, _) = broadcast::channel(256);
    let device_connected = Arc::new(AtomicBool::new(true));
    let client_connected = Arc::new(AtomicBool::new(true));

    Self {
      name: name.to_owned(),
      index,
      device_messages,
      event_loop_sender: message_sender,
      internal_event_sender: event_sender,
      device_connected,
      client_connected,
    }
  }

  pub(super) fn new_from_device_fields(
    device_index: u32,
    device_name: &str,
    device_messages: &Vec<ButtplugDeviceMessageNameV0>,
    sender: mpsc::Sender<ButtplugClientRequest>,
  ) -> Self {
    ButtplugClientDevice::new(device_name, device_index, device_messages.clone(), sender)
  }

  pub fn connected(&self) -> bool {
    self.device_connected.load(Ordering::Relaxed)
  }

  /// Sends a message through the owning
  /// [ButtplugClient][super::ButtplugClient].
  ///
  /// Performs the send/receive flow for send a device command and receiving the
  /// response from the server.
  fn send_message(
    &self,
    msg: ButtplugClientMessageV0,
  ) -> ButtplugClientResultFuture<ButtplugServerMessageV0> {
    let message_sender = self.event_loop_sender.clone();
    let client_connected = self.client_connected.clone();
    let device_connected = self.device_connected.clone();
    let id = msg.id();
    let device_name = self.name.clone();
    Box::pin(
      async move {
        if !client_connected.load(Ordering::Relaxed) {
          error!("Client not connected, cannot run device command");
          return Err(ButtplugConnectorError::ConnectorNotConnected.into());
        } else if !device_connected.load(Ordering::Relaxed) {
          error!("Device not connected, cannot run device command");
          return Err(
            ButtplugError::from(ButtplugDeviceError::DeviceNotConnected(device_name)).into(),
          );
        }
        let (tx, rx) = oneshot::channel();
        message_sender
          .send(ButtplugClientRequest::Message(
            ButtplugClientMessageFuturePair::new(msg.clone(), tx),
          ))
          .await
          .map_err(|_| {
            ButtplugClientError::ButtplugConnectorError(
              ButtplugConnectorError::ConnectorChannelClosed,
            )
          })?;
        let msg = rx
          .await
          .map_err(|_| ButtplugConnectorError::ConnectorChannelClosed)??;
        if let ButtplugServerMessageV0::Error(_err) = msg {
          Err(ButtplugError::from(_err).into())
        } else {
          Ok(msg)
        }
      }
      .instrument(tracing::trace_span!("ClientDeviceSendFuture for {}", id)),
    )
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
    Box::pin(future::ready(Err(ButtplugClientError::ButtplugError(err))))
  }

  /// Sends a message, expecting back an [Ok][crate::core::messages::Ok]
  /// message, otherwise returns a [ButtplugError]
  fn send_message_expect_ok(&self, msg: ButtplugClientMessageV0) -> ButtplugClientResultFuture {
    let send_fut = self.send_message(msg);
    Box::pin(async move {
      match send_fut.await? {
        ButtplugServerMessageV0::Ok(_) => Ok(()),
        ButtplugServerMessageV0::Error(_err) => Err(ButtplugError::from(_err).into()),
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

  /// Commands device to vibrate at a single speed (all motors).
  pub fn single_motor_vibrate(&self, speed: f64) -> ButtplugClientResultFuture {
    self.send_message_expect_ok(SingleMotorVibrateCmdV0::new(self.index, speed).into())
  }

  /// Commands device to stop all movement.
  pub fn stop(&self) -> ButtplugClientResultFuture {
    // All devices accept StopDeviceCmd
    self.send_message_expect_ok(StopDeviceCmdV0::new(self.index).into())
  }

  pub fn index(&self) -> u32 {
    self.index
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

impl Eq for ButtplugClientDevice {}

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
