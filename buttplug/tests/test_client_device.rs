mod util;
use buttplug::{
  client::{ButtplugClient, ButtplugClientEvent, ButtplugClientDeviceEvent, VibrateCommand},
  connector::ButtplugInProcessClientConnector, 
  util::async_manager
};
use futures::{pin_mut, StreamExt};
use futures_timer::Delay;
use std::time::Duration;

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
    client
      .connect(connector)
      .await
      .unwrap();
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
    client
      .connect(connector)
      .await
      .unwrap();
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
    client
      .connect(connector)
      .await
      .unwrap();
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
    client
      .connect(connector)
      .await
      .unwrap();
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
    assert!(test_device.vibrate(VibrateCommand::Speed(2.0)).await.is_err());
  });
}

// TODO Test invalid messages to device
// TODO Test invalid parameters in message
// TODO Test device invalidation across client connections (i.e. a device shouldn't be allowed to reconnect even if index is the same)
// TODO Test DeviceAdded being sent multiple times w/ same index
// TODO Test DeviceRemoved being sent multiple times
// TODO Test DeviceList being sent followed by repeat DeviceAdded
// TODO Test DeviceList being sent multiple times
// TODO Test sending device return for device that doesn't exist (in client)