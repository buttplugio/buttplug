mod util;
use buttplug::{
  client::{ButtplugClient, ButtplugClientEvent, ButtplugClientDeviceEvent},
  connector::ButtplugInProcessClientConnector, 
  util::async_manager
};
use futures::{pin_mut, StreamExt};
use tracing_subscriber;

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