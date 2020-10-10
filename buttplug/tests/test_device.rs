mod util;
use buttplug::{
  core::{
    messages::{
      self,
      ButtplugDeviceMessageType,
      ButtplugServerMessage,
      BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
    },
  },
  server::ButtplugServer,
  util::async_manager,
};
use futures::StreamExt;

// Test devices that have protocols that support movements not all devices do.
// For instance, the Onyx+ is part of a protocol that supports vibration, but
// the device itself does not.
#[test]
fn test_capabilities_exposure() {
  async_manager::block_on(async {
    let (server, mut recv) = ButtplugServer::new_with_defaults();
    let helper = server.add_test_comm_manager().unwrap();
    helper.add_ble_device("Onyx+").await;
    server
      .parse_message(
        messages::RequestServerInfo::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION)
          .into()
      )
      .await
      .unwrap();
    server
      .parse_message(messages::StartScanning::default().into())
      .await
      .unwrap();
    while let Some(msg) = recv.next().await {
      if let ButtplugServerMessage::DeviceAdded(device) = msg {
        assert!(!device.device_messages.contains_key(&ButtplugDeviceMessageType::VibrateCmd));
        assert!(!device.device_messages.contains_key(&ButtplugDeviceMessageType::SingleMotorVibrateCmd));
        assert!(device.device_messages.contains_key(&ButtplugDeviceMessageType::LinearCmd));
        assert!(device.device_messages.contains_key(&ButtplugDeviceMessageType::StopDeviceCmd));
        return;
      }
    }
  });
}