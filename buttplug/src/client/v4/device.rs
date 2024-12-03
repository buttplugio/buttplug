// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Representation and management of devices connected to the server.

use super::{
  client_device_feature::ClientDeviceFeature,
  create_boxed_future_client_error,
  ButtplugClientMessageSender,
  ButtplugClientResultFuture,
};
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
    message::{
      ButtplugClientMessageV4,
      ButtplugServerMessageV4,
      DeviceFeature,
      DeviceMessageInfoV4,
      Endpoint,
      FeatureType,
      LevelCmdV4,
      RawReadCmdV2,
      RawSubscribeCmdV2,
      RawUnsubscribeCmdV2,
      RawWriteCmdV2,
      StopDeviceCmdV0,
    },
  },
  util::stream::convert_broadcast_receiver_to_stream,
};
use futures::{FutureExt, Stream};
use getset::{CopyGetters, Getters};
use std::{
  fmt,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
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
  device_features: Vec<ClientDeviceFeature>,
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
    device_features: &Vec<DeviceFeature>,
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
      device_features: device_features
        .iter()
        .enumerate()
        .map(|(i, x)| ClientDeviceFeature::new(index, i as u32, x, &message_sender))
        .collect(),
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
    self.device_connected.load(Ordering::SeqCst)
  }

  pub fn event_stream(&self) -> Box<dyn Stream<Item = ButtplugClientDeviceEvent> + Send + Unpin> {
    Box::new(Box::pin(convert_broadcast_receiver_to_stream(
      self.internal_event_sender.subscribe(),
    )))
  }

  fn filter_device_features(&self, feature_type: FeatureType) -> Vec<ClientDeviceFeature> {
    self
      .device_features
      .iter()
      .filter(|x| *x.feature().feature_type() == feature_type)
      .cloned()
      .collect()
  }

  fn level(&self, feature_type: FeatureType, level: i32) -> ButtplugClientResultFuture {
    let features = self.filter_device_features(feature_type);
    if features.is_empty() {
      // TODO err
    }
    let subcommands = features
      .iter()
      .map(|x| x.level_subcommand(level as i32))
      .collect();
    let command = LevelCmdV4::new(self.index, subcommands);
    self
      .event_loop_sender
      .send_message_expect_ok(command.into())
      .into()
  }

  pub fn vibrate_features(&self) -> Vec<ClientDeviceFeature> {
    self.filter_device_features(FeatureType::Vibrate)
  }

  /// Commands device to vibrate, assuming it has the features to do so.
  pub fn vibrate(&self, speed: u32) -> ButtplugClientResultFuture {
    self.level(FeatureType::Vibrate, speed as i32)
  }

  pub fn has_battery_level(&self) -> bool {
    self
      .device_features
      .iter()
      .find(|x| *x.feature().feature_type() == FeatureType::Battery)
      .is_some()
  }

  pub fn battery_level(&self) -> ButtplugClientResultFuture<u32> {
    if let Some(battery) = self
      .device_features
      .iter()
      .find(|x| *x.feature().feature_type() == FeatureType::Battery)
    {
      battery.battery_level()
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
      .find(|x| *x.feature().feature_type() == FeatureType::RSSI)
      .is_some()
  }

  pub fn rssi_level(&self) -> ButtplugClientResultFuture<u32> {
    if let Some(rssi) = self
      .device_features
      .iter()
      .find(|x| *x.feature().feature_type() == FeatureType::RSSI)
    {
      rssi.rssi_level()
    } else {
      create_boxed_future_client_error(
        ButtplugDeviceError::DeviceFeatureMismatch(
          "Device does not have RSSI feature available".to_owned(),
        )
        .into(),
      )
    }
  }

  pub fn raw_write(
    &self,
    endpoint: Endpoint,
    data: &[u8],
    write_with_response: bool,
  ) -> ButtplugClientResultFuture {
    if self
      .device_features
      .iter()
      .find(|x| x.feature().raw().is_some())
      .is_some()
    {
      let msg = ButtplugClientMessageV4::RawWriteCmd(RawWriteCmdV2::new(
        self.index,
        endpoint,
        data,
        write_with_response,
      ));
      self.event_loop_sender.send_message_expect_ok(msg)
    } else {
      create_boxed_future_client_error(
        ButtplugDeviceError::DeviceFeatureMismatch(
          "Device does not have raw feature available".to_owned(),
        )
        .into(),
      )
    }
  }

  pub fn raw_read(
    &self,
    endpoint: Endpoint,
    expected_length: u32,
    timeout: u32,
  ) -> ButtplugClientResultFuture<Vec<u8>> {
    if self
      .device_features
      .iter()
      .find(|x| x.feature().raw().is_some())
      .is_some()
    {
      let msg = ButtplugClientMessageV4::RawReadCmd(RawReadCmdV2::new(
        self.index,
        endpoint,
        expected_length,
        timeout,
      ));
      let send_fut = self.event_loop_sender.send_message(msg);
      async move {
        match send_fut.await? {
          ButtplugServerMessageV4::RawReading(reading) => Ok(reading.data().clone()),
          ButtplugServerMessageV4::Error(err) => Err(ButtplugError::from(err).into()),
          msg => Err(
            ButtplugError::from(ButtplugMessageError::UnexpectedMessageType(format!(
              "{:?}",
              msg
            )))
            .into(),
          ),
        }
      }
      .boxed()
    } else {
      create_boxed_future_client_error(
        ButtplugDeviceError::DeviceFeatureMismatch(
          "Device does not have raw feature available".to_owned(),
        )
        .into(),
      )
    }
  }

  pub fn raw_subscribe(&self, endpoint: Endpoint) -> ButtplugClientResultFuture {
    if self
      .device_features
      .iter()
      .find(|x| x.feature().raw().is_some())
      .is_some()
    {
      let msg =
        ButtplugClientMessageV4::RawSubscribeCmd(RawSubscribeCmdV2::new(self.index, endpoint));
      self.event_loop_sender.send_message_expect_ok(msg)
    } else {
      create_boxed_future_client_error(
        ButtplugDeviceError::DeviceFeatureMismatch(
          "Device does not have raw feature available".to_owned(),
        )
        .into(),
      )
    }
  }

  pub fn raw_unsubscribe(&self, endpoint: Endpoint) -> ButtplugClientResultFuture {
    if self
      .device_features
      .iter()
      .find(|x| x.feature().raw().is_some())
      .is_some()
    {
      let msg =
        ButtplugClientMessageV4::RawUnsubscribeCmd(RawUnsubscribeCmdV2::new(self.index, endpoint));
      self.event_loop_sender.send_message_expect_ok(msg)
    } else {
      create_boxed_future_client_error(
        ButtplugDeviceError::DeviceFeatureMismatch(
          "Device does not have raw feature available".to_owned(),
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
