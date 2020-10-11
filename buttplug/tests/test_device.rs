mod util;
use buttplug::{
  core::{
    errors::{ButtplugError, ButtplugDeviceError, ButtplugServerError},
    messages::{
      self,
      ButtplugDeviceMessageType,
      ButtplugServerMessage,
      BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
    },
  },
  device::Endpoint,
  server::{ButtplugServer, ButtplugServerOptions},
  util::async_manager,
};
use std::matches;
use futures::StreamExt;

// Test devices that have protocols that support movements not all devices do.
// For instance, the Onyx+ is part of a protocol that supports vibration, but
// the device itself does not.
#[test]
fn test_capabilities_exposure() {
  async_manager::block_on(async {
    let (server, mut recv) = ButtplugServer::default();
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


#[test]
fn test_server_raw_message() {
  async_manager::block_on(async {
    let mut options = ButtplugServerOptions::default();
    options.allow_raw_messages = true;
    let (server, mut recv) = ButtplugServer::new_with_options(options).unwrap();
    let helper = server.add_test_comm_manager().unwrap();
    helper.add_ble_device("Massage Demo").await;
    assert!(server
      .parse_message(
        messages::RequestServerInfo::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION)
          .into()
      )
      .await
      .is_ok());
    assert!(server
      .parse_message(messages::StartScanning::default().into())
      .await
      .is_ok());
    while let Some(msg) = recv.next().await {
      if let ButtplugServerMessage::ScanningFinished(_) = msg {
        continue;
      } else if let ButtplugServerMessage::DeviceAdded(da) = msg {
        assert_eq!(da.device_name, "Aneros Vivi");
        assert!(da.device_messages.contains_key(&ButtplugDeviceMessageType::RawReadCmd));
        assert!(da.device_messages.contains_key(&ButtplugDeviceMessageType::RawWriteCmd));
        assert!(da.device_messages.contains_key(&ButtplugDeviceMessageType::RawSubscribeCmd));
        assert!(da.device_messages.contains_key(&ButtplugDeviceMessageType::RawUnsubscribeCmd));
        return;
      } else {
        panic!(format!(
          "Returned message was not a DeviceAdded message or timed out: {:?}",
          msg
        ));
      }
    }
  });
}

#[test]
fn test_server_no_raw_message() {
  async_manager::block_on(async {
    let (server, mut recv) = ButtplugServer::default();
    let helper = server.add_test_comm_manager().unwrap();
    helper.add_ble_device("Massage Demo").await;
    assert!(server
      .parse_message(
        messages::RequestServerInfo::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION)
          .into()
      )
      .await
      .is_ok());
    assert!(server
      .parse_message(messages::StartScanning::default().into())
      .await
      .is_ok());
    while let Some(msg) = recv.next().await {
      if let ButtplugServerMessage::ScanningFinished(_) = msg {
        continue;
      } else if let ButtplugServerMessage::DeviceAdded(da) = msg {
        assert_eq!(da.device_name, "Aneros Vivi");
        assert!(!da.device_messages.contains_key(&ButtplugDeviceMessageType::RawReadCmd));
        assert!(!da.device_messages.contains_key(&ButtplugDeviceMessageType::RawWriteCmd));
        assert!(!da.device_messages.contains_key(&ButtplugDeviceMessageType::RawSubscribeCmd));
        assert!(!da.device_messages.contains_key(&ButtplugDeviceMessageType::RawUnsubscribeCmd));
        return;
      } else {
        panic!(format!(
          "Returned message was not a DeviceAdded message or timed out: {:?}",
          msg
        ));
      }
    }
  });
}

#[test]
fn test_reject_on_no_raw_message() {
  async_manager::block_on(async {
    let (server, mut recv) = ButtplugServer::default();
    let helper = server.add_test_comm_manager().unwrap();
    helper.add_ble_device("Massage Demo").await;
    assert!(server
      .parse_message(
        messages::RequestServerInfo::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION)
          .into()
      )
      .await
      .is_ok());
    assert!(server
      .parse_message(messages::StartScanning::default().into())
      .await
      .is_ok());
    while let Some(msg) = recv.next().await {
      if let ButtplugServerMessage::ScanningFinished(_) = msg {
        continue;
      } else if let ButtplugServerMessage::DeviceAdded(da) = msg {
        assert_eq!(da.device_name, "Aneros Vivi");
        let mut should_be_err;
        should_be_err = server.parse_message(messages::RawWriteCmd::new(da.device_index, Endpoint::Tx, vec![0x0], false).into()).await;
        assert!(should_be_err.is_err());
        assert!(matches!(should_be_err.err().unwrap().error(), ButtplugError::ButtplugDeviceError(ButtplugDeviceError::MessageNotSupported(_))));

        should_be_err = server.parse_message(messages::RawReadCmd::new(da.device_index, Endpoint::Tx, 0, 0).into()).await;
        assert!(should_be_err.is_err());
        assert!(matches!(should_be_err.err().unwrap().error(), ButtplugError::ButtplugDeviceError(ButtplugDeviceError::MessageNotSupported(_))));

        should_be_err = server.parse_message(messages::RawSubscribeCmd::new(da.device_index, Endpoint::Tx).into()).await;
        assert!(should_be_err.is_err());
        assert!(matches!(should_be_err.err().unwrap().error(), ButtplugError::ButtplugDeviceError(ButtplugDeviceError::MessageNotSupported(_))));

        should_be_err = server.parse_message(messages::RawUnsubscribeCmd::new(da.device_index, Endpoint::Tx).into()).await;
        assert!(should_be_err.is_err());
        assert!(matches!(should_be_err.err().unwrap().error(), ButtplugError::ButtplugDeviceError(ButtplugDeviceError::MessageNotSupported(_))));
        return;
      } else {
        panic!(format!(
          "Returned message was not a DeviceAdded message or timed out: {:?}",
          msg
        ));
      }
    }
  });
}
