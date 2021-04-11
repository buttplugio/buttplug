mod util;
use buttplug::{
  client::{
    ButtplugClient, ButtplugClientDeviceEvent, ButtplugClientError, ButtplugClientEvent,
    VibrateCommand,
  },
  connector::ButtplugInProcessClientConnector,
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
    messages::{self, ButtplugClientMessage},
  },
  util::async_manager,
};
use futures::{pin_mut, StreamExt};
use futures_timer::Delay;
use std::{collections::HashMap, sync::Arc, time::Duration};

#[cfg(feature = "server")]
#[test]
fn test_client_device_connected_status() {
  async_manager::block_on(async {
    let client = ButtplugClient::new("Test Client");
    let mut event_stream = client.event_stream();
    let connector = ButtplugInProcessClientConnector::default();
    let helper = connector.server_ref().add_test_comm_manager().unwrap();
    let recv = client.event_stream();
    pin_mut!(recv);
    let device = helper.add_ble_device("Massage Demo").await;
    assert!(!client.connected());
    client.connect(connector).await.unwrap();
    assert!(client.connected());
    client.start_scanning().await.unwrap();
    let mut client_device = None;
    while let Some(msg) = event_stream.next().await {
      if let ButtplugClientEvent::DeviceAdded(da) = msg {
        client_device = Some(da);
        break;
      }
    }
    let test_device = client_device.unwrap();
    let mut device_event_stream = test_device.event_stream();
    assert!(test_device.connected());
    device.disconnect().await.unwrap();
    while let Some(msg) = device_event_stream.next().await {
      if let ButtplugClientDeviceEvent::DeviceRemoved = msg {
        assert!(!test_device.connected());
        break;
      }
    }
    client.disconnect().await.unwrap();
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
    let helper = connector.server_ref().add_test_comm_manager().unwrap();
    let recv = client.event_stream();
    pin_mut!(recv);
    let _ = helper.add_ble_device("Massage Demo").await;
    assert!(!client.connected());
    client.connect(connector).await.unwrap();
    assert!(client.connected());
    client.start_scanning().await.unwrap();
    let mut client_device = None;
    while let Some(msg) = event_stream.next().await {
      if let ButtplugClientEvent::DeviceAdded(da) = msg {
        client_device = Some(da);
        break;
      }
    }
    let test_device = client_device.unwrap();
    let mut device_event_stream = test_device.event_stream();
    assert!(test_device.connected());
    client.disconnect().await.unwrap();
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
    let helper = connector.server_ref().add_test_comm_manager().unwrap();
    let device = helper.add_ble_device("Massage Demo").await;
    assert!(!client.connected());
    client.connect(connector).await.unwrap();
    assert!(client.connected());
    client.start_scanning().await.unwrap();
    Delay::new(Duration::from_millis(100)).await;
    device.disconnect().await.unwrap();
    Delay::new(Duration::from_millis(100)).await;
    client.disconnect().await.unwrap();
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
    let helper = connector.server_ref().add_test_comm_manager().unwrap();
    let recv = client.event_stream();
    pin_mut!(recv);
    let _ = helper.add_ble_device("Massage Demo").await;
    assert!(!client.connected());
    client.connect(connector).await.unwrap();
    assert!(client.connected());
    client.start_scanning().await.unwrap();
    let mut client_device = None;
    while let Some(msg) = event_stream.next().await {
      if let ButtplugClientEvent::DeviceAdded(da) = msg {
        client_device = Some(da);
        break;
      }
    }
    let test_device = client_device.unwrap();
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
        helper_clone.get_next_client_message().await,
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
    })
    .unwrap();
    helper.client().start_scanning().await.unwrap();
    assert!(matches!(
      event_stream.next().await.unwrap(),
      ButtplugClientEvent::DeviceAdded(..)
    ));
    assert!(matches!(
      event_stream.next().await.unwrap(),
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
        helper_clone.get_next_client_message().await,
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
    })
    .unwrap();
    helper.client().start_scanning().await.unwrap();
    assert!(matches!(
      event_stream.next().await.unwrap(),
      ButtplugClientEvent::DeviceAdded(..)
    ));
    assert!(matches!(
      event_stream.next().await.unwrap(),
      ButtplugClientEvent::DeviceRemoved(..)
    ));
    assert!(matches!(
      event_stream.next().await.unwrap(),
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
