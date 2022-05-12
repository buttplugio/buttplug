// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;
use buttplug::{
  client::{
    ButtplugClient,
    ButtplugClientDeviceEvent,
    ButtplugClientError,
    ButtplugClientEvent,
    VibrateCommand,
  },
  core::{
    connector::ButtplugInProcessClientConnector,
    errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
    messages::{self, ButtplugClientMessage},
  },
  server::device::communication_manager::test::TestDeviceCommunicationManagerBuilder,
  util::async_manager,
};
use futures::StreamExt;
use futures_timer::Delay;
use std::{collections::HashMap, sync::Arc, time::Duration};

#[cfg(feature = "server")]
#[test]
fn test_client_device_connected_status() {
  async_manager::block_on(async {
    let client = ButtplugClient::new("Test Client");
    let mut event_stream = client.event_stream();
    let connector = ButtplugInProcessClientConnector::default();
    let builder = TestDeviceCommunicationManagerBuilder::default();
    let helper = builder.helper();
    connector
      .server_ref()
      .device_manager()
      .add_comm_manager(builder)
      .expect("Test, assuming infallible.");
    let device = helper.add_ble_device("Massage Demo").await;
    assert!(!client.connected());
    client
      .connect(connector)
      .await
      .expect("Test, assuming infallible.");
    assert!(client.connected());
    client
      .start_scanning()
      .await
      .expect("Test, assuming infallible.");
    let mut client_device = None;
    while let Some(msg) = event_stream.next().await {
      if let ButtplugClientEvent::DeviceAdded(da) = msg {
        client_device = Some(da);
        break;
      }
    }
    let test_device = client_device.expect("Test, assuming infallible.");
    let mut device_event_stream = test_device.event_stream();
    assert!(test_device.connected());
    device
      .disconnect()
      .await
      .expect("Test, assuming infallible.");
    while let Some(msg) = device_event_stream.next().await {
      if let ButtplugClientDeviceEvent::DeviceRemoved = msg {
        assert!(!test_device.connected());
        break;
      }
    }
    client
      .disconnect()
      .await
      .expect("Test, assuming infallible.");
    assert!(!client.connected());
  });
}

#[cfg(feature = "server")]
#[test]
fn test_client_device_client_disconnected_status() {
  async_manager::block_on(async {
    let client = ButtplugClient::new("Test Client");
    let mut event_stream = client.event_stream();
    let connector = ButtplugInProcessClientConnector::default();
    let builder = TestDeviceCommunicationManagerBuilder::default();
    let helper = builder.helper();
    connector
      .server_ref()
      .device_manager()
      .add_comm_manager(builder)
      .expect("Test, assuming infallible.");
    let _ = helper.add_ble_device("Massage Demo").await;
    assert!(!client.connected());
    client
      .connect(connector)
      .await
      .expect("Test, assuming infallible.");
    assert!(client.connected());
    client
      .start_scanning()
      .await
      .expect("Test, assuming infallible.");
    let mut client_device = None;
    while let Some(msg) = event_stream.next().await {
      if let ButtplugClientEvent::DeviceAdded(da) = msg {
        client_device = Some(da);
        break;
      }
    }
    let test_device = client_device.expect("Test, assuming infallible.");
    let mut device_event_stream = test_device.event_stream();
    assert!(test_device.connected());
    client
      .disconnect()
      .await
      .expect("Test, assuming infallible.");
    while let Some(msg) = event_stream.next().await {
      if let ButtplugClientEvent::ServerDisconnect = msg {
        assert!(!client.connected());
        assert!(!test_device.connected());
        break;
      }
    }
    while let Some(msg) = device_event_stream.next().await {
      if let ButtplugClientDeviceEvent::DeviceRemoved = msg {
        break;
      }
    }
  });
}

#[cfg(feature = "server")]
#[test]
fn test_client_device_connected_no_event_listener() {
  async_manager::block_on(async {
    let client = ButtplugClient::new("Test Client");
    let connector = ButtplugInProcessClientConnector::default();
    let builder = TestDeviceCommunicationManagerBuilder::default();
    let helper = builder.helper();
    connector
      .server_ref()
      .device_manager()
      .add_comm_manager(builder)
      .expect("Test, assuming infallible.");
    let device = helper.add_ble_device("Massage Demo").await;
    assert!(!client.connected());
    client
      .connect(connector)
      .await
      .expect("Test, assuming infallible.");
    assert!(client.connected());
    client
      .start_scanning()
      .await
      .expect("Test, assuming infallible.");
    Delay::new(Duration::from_millis(100)).await;
    device
      .disconnect()
      .await
      .expect("Test, assuming infallible.");
    Delay::new(Duration::from_millis(100)).await;
    client
      .disconnect()
      .await
      .expect("Test, assuming infallible.");
    assert!(!client.connected());
    Delay::new(Duration::from_millis(100)).await;
  });
}

#[cfg(feature = "server")]
#[test]
fn test_client_device_invalid_command() {
  async_manager::block_on(async {
    let client = ButtplugClient::new("Test Client");
    let mut event_stream = client.event_stream();
    let connector = ButtplugInProcessClientConnector::default();
    let builder = TestDeviceCommunicationManagerBuilder::default();
    let helper = builder.helper();
    connector
      .server_ref()
      .device_manager()
      .add_comm_manager(builder)
      .expect("Test, assuming infallible.");
    let _ = helper.add_ble_device("Massage Demo").await;
    assert!(!client.connected());
    client
      .connect(connector)
      .await
      .expect("Test, assuming infallible.");
    assert!(client.connected());
    client
      .start_scanning()
      .await
      .expect("Test, assuming infallible.");
    let mut client_device = None;
    while let Some(msg) = event_stream.next().await {
      if let ButtplugClientEvent::DeviceAdded(da) = msg {
        client_device = Some(da);
        break;
      }
    }
    let test_device = client_device.expect("Test, assuming infallible.");
    assert!(matches!(
      test_device
        .vibrate(VibrateCommand::Speed(2.0))
        .await
        .unwrap_err(),
      ButtplugClientError::ButtplugError(ButtplugError::ButtplugMessageError(
        ButtplugMessageError::InvalidMessageContents(..)
      ))
    ));
    assert!(matches!(
      test_device
        .vibrate(VibrateCommand::SpeedVec(vec!(0.5, 0.5, 0.5)))
        .await
        .unwrap_err(),
      ButtplugClientError::ButtplugError(ButtplugError::ButtplugDeviceError(
        ButtplugDeviceError::DeviceFeatureCountMismatch(..)
      ))
    ));
    assert!(matches!(
      test_device
        .vibrate(VibrateCommand::SpeedVec(vec!()))
        .await
        .unwrap_err(),
      ButtplugClientError::ButtplugError(ButtplugError::ButtplugDeviceError(
        ButtplugDeviceError::ProtocolRequirementError(..)
      ))
    ));
  });
}

#[cfg(feature = "server")]
#[test]
fn test_client_repeated_deviceadded_message() {
  async_manager::block_on(async move {
    let helper = Arc::new(util::ChannelClientTestHelper::new());
    helper.simulate_successful_connect().await;
    let helper_clone = helper.clone();
    let mut event_stream = helper.client().event_stream();
    async_manager::spawn(async move {
      assert!(matches!(
        helper_clone.next_client_message().await,
        ButtplugClientMessage::StartScanning(..)
      ));
      helper_clone
        .send_client_incoming(messages::Ok::new(3).into())
        .await;
      let device_added = messages::DeviceAdded::new(1, "Test Device", &HashMap::new());
      helper_clone
        .send_client_incoming(device_added.clone().into())
        .await;
      helper_clone.send_client_incoming(device_added.into()).await;
    });
    helper
      .client()
      .start_scanning()
      .await
      .expect("Test, assuming infallible.");
    assert!(matches!(
      event_stream
        .next()
        .await
        .expect("Test, assuming infallible."),
      ButtplugClientEvent::DeviceAdded(..)
    ));
    assert!(matches!(
      event_stream
        .next()
        .await
        .expect("Test, assuming infallible."),
      ButtplugClientEvent::Error(..)
    ));
  });
}

#[cfg(feature = "server")]
#[test]
fn test_client_repeated_deviceremoved_message() {
  async_manager::block_on(async move {
    let helper = Arc::new(util::ChannelClientTestHelper::new());
    helper.simulate_successful_connect().await;
    let helper_clone = helper.clone();
    let mut event_stream = helper.client().event_stream();
    async_manager::spawn(async move {
      assert!(matches!(
        helper_clone.next_client_message().await,
        ButtplugClientMessage::StartScanning(..)
      ));
      helper_clone
        .send_client_incoming(messages::Ok::new(3).into())
        .await;
      let device_added = messages::DeviceAdded::new(1, "Test Device", &HashMap::new());
      let device_removed = messages::DeviceRemoved::new(1);
      helper_clone.send_client_incoming(device_added.into()).await;
      helper_clone
        .send_client_incoming(device_removed.clone().into())
        .await;
      helper_clone
        .send_client_incoming(device_removed.into())
        .await;
    });
    helper
      .client()
      .start_scanning()
      .await
      .expect("Test, assuming infallible.");
    assert!(matches!(
      event_stream
        .next()
        .await
        .expect("Test, assuming infallible."),
      ButtplugClientEvent::DeviceAdded(..)
    ));
    assert!(matches!(
      event_stream
        .next()
        .await
        .expect("Test, assuming infallible."),
      ButtplugClientEvent::DeviceRemoved(..)
    ));
    assert!(matches!(
      event_stream
        .next()
        .await
        .expect("Test, assuming infallible."),
      ButtplugClientEvent::Error(..)
    ));
  });
}

// TODO Test invalid messages to device
// TODO Test invalid parameters in message
// TODO Test device invalidation across client connections (i.e. a device shouldn't be allowed to reconnect even if index is the same)
// TODO Test DeviceList being sent followed by repeat DeviceAdded
// TODO Test DeviceList being sent multiple times
// TODO Test sending device return for device that doesn't exist (in client)
