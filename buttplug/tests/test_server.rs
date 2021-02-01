mod util;

use buttplug::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugHandshakeError},
    messages::{
      self,
      ButtplugMessageSpecVersion,
      ButtplugServerMessage,
      BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
    },
  },
  device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
  server::{ButtplugServer, ButtplugServerOptions},
  test::check_test_recv_value,
  util::async_manager,
};
use futures::{pin_mut, Stream, StreamExt};
use futures_timer::Delay;
use std::time::Duration;

async fn setup_test_server(
  msg_union: messages::ButtplugClientMessage,
) -> (ButtplugServer, impl Stream<Item = ButtplugServerMessage>) {
  let server = ButtplugServer::default();
  let recv = server.event_stream();
  // assert_eq!(server.server_name, "Test Server");
  match server.parse_message(msg_union).await.unwrap() {
    ButtplugServerMessage::ServerInfo(s) => assert_eq!(
      s,
      messages::ServerInfo::new("Buttplug Server", ButtplugMessageSpecVersion::Version2, 0)
    ),
    _ => panic!("Should've received ok"),
  }
  (server, recv)
}

#[test]
fn test_server_handshake() {
  let msg =
    messages::RequestServerInfo::new("Test Client", ButtplugMessageSpecVersion::Version2).into();
  async_manager::block_on(async {
    let (server, _recv) = setup_test_server(msg).await;
    assert!(server.connected());
  });
}

#[test]
fn test_server_handshake_not_done_first() {
  let msg = messages::Ping::default().into();
  async_manager::block_on(async {
    let server = ButtplugServer::default();
    // assert_eq!(server.server_name, "Test Server");
    let result = server.parse_message(msg).await;
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err().original_error(),
      ButtplugError::ButtplugHandshakeError(ButtplugHandshakeError::RequestServerInfoExpected)
    ));
    assert!(!server.connected());
  });
}

#[test]
fn test_server_version_lt() {
  let msg =
    messages::RequestServerInfo::new("Test Client", ButtplugMessageSpecVersion::Version2).into();
  async_manager::block_on(async {
    let _ = setup_test_server(msg).await;
  });
}

// TODO Now that we're moving to a spec version enum, this test is invalid
// because we can't just pass a u8 in. This should be rebuilt using the
// JSON parser, and it should fail to deserialize the message.
#[test]
#[ignore]
fn test_server_version_gt() {
  let server = ButtplugServer::default();
  let msg =
    messages::RequestServerInfo::new("Test Client", ButtplugMessageSpecVersion::Version2).into();
  async_manager::block_on(async {
    assert!(
      server.parse_message(msg).await.is_err(),
      "Client having higher version than server should fail"
    );
  });
}

#[test]
fn test_ping_timeout() {
  let mut options = ButtplugServerOptions::default();
  options.max_ping_time = 100;
  let server = ButtplugServer::new_with_options(&options).unwrap();
  let recv = server.event_stream();
  pin_mut!(recv);
  async_manager::block_on(async {
    let msg =
      messages::RequestServerInfo::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION);
    Delay::new(Duration::from_millis(150)).await;
    let reply = server.parse_message(msg.into()).await;
    assert!(
      reply.is_ok(),
      format!(
        "ping timer shouldn't start until handshake finished. {:?}",
        reply
      )
    );
    Delay::new(Duration::from_millis(300)).await;
    let pingmsg = messages::Ping::default();
    let result = server.parse_message(pingmsg.into()).await;
    let err = result.unwrap_err();
    if !matches!(err.original_error(), ButtplugError::ButtplugPingError(_)) {
      panic!("Got wrong type of error back!");
    }
    // Check that we got an event back about the ping out.
    let msg = recv.next().await.unwrap();
    if let ButtplugServerMessage::Error(e) = msg {
      if messages::ErrorCode::ErrorPing != e.error_code {
        panic!("Didn't get a ping error");
      }
    } else {
      panic!("Didn't get an error message back");
    }
  });
}

#[test]
fn test_device_stop_on_ping_timeout() {
  async_manager::block_on(async {
    let mut options = ButtplugServerOptions::default();
    options.max_ping_time = 100;
    let server = ButtplugServer::new_with_options(&options).unwrap();
    let recv = server.event_stream();
    pin_mut!(recv);
    let helper = server.add_test_comm_manager().unwrap();
    // TODO This should probably use a test protocol we control, not the aneros protocol
    let device = helper.add_ble_device("Massage Demo").await;

    let msg =
      messages::RequestServerInfo::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION);
    let mut reply = server.parse_message(msg.into()).await;
    assert!(reply.is_ok());
    reply = server
      .parse_message(messages::StartScanning::default().into())
      .await;
    assert!(reply.is_ok());
    // Check that we got an event back about a new device.
    let mut device_index = 100;
    while let Some(msg) = recv.next().await {
      if let ButtplugServerMessage::ScanningFinished(_) = msg {
        continue;
      } else if let ButtplugServerMessage::DeviceAdded(da) = msg {
        assert_eq!(da.device_name(), "Aneros Vivi");
        device_index = da.device_index();
        break;
      } else {
        panic!(format!(
          "Returned message was not a DeviceAdded message or timed out: {:?}",
          msg
        ));
      }
    }
    server
      .parse_message(
        messages::VibrateCmd::new(device_index, vec![messages::VibrateSubcommand::new(0, 0.5)])
          .into(),
      )
      .await
      .unwrap();
    let command_receiver = device.get_endpoint_receiver(&Endpoint::Tx).unwrap();
    check_test_recv_value(
      &command_receiver,
      DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 64], false)),
    );
    /*
    // Wait out the ping, we should get a stop message.
    let mut i = 0u32;
    while command_receiver.is_empty() {
      Delay::new(Duration::from_millis(150)).await;
      // Breaks out of loop if we wait for too long.
      i += 1;
      assert!(i < 10, "Slept for too long while waiting for stop command!");
    }
    check_test_recv_value(
      &command_receiver,
      DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 0], false)),
    );
    */
  });
}

#[test]
fn test_repeated_handshake() {
  let msg = messages::RequestServerInfo::new("Test Client", ButtplugMessageSpecVersion::Version2);
  async_manager::block_on(async {
    let (server, _recv) = setup_test_server((msg.clone()).into()).await;
    assert!(server.connected());
    let err = server.parse_message(msg.into()).await.unwrap_err();
    assert!(matches!(
      err.original_error(),
      ButtplugError::ButtplugHandshakeError(ButtplugHandshakeError::HandshakeAlreadyHappened)
    ));
  });
}

#[test]
fn test_invalid_device_index() {
  async_manager::block_on(async {
    let msg =
      messages::RequestServerInfo::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION);
    let (server, _) = setup_test_server(msg.into()).await;
    let reply = server
      .parse_message(messages::VibrateCmd::new(10, vec![]).into())
      .await;
    assert!(reply.is_err());
    assert!(matches!(
      reply.unwrap_err().original_error(),
      ButtplugError::ButtplugDeviceError(ButtplugDeviceError::DeviceNotAvailable(_))
    ));
  });
}

#[test]
fn test_device_index_generation() {
  async_manager::block_on(async {
    let server = ButtplugServer::default();
    let recv = server.event_stream();
    pin_mut!(recv);
    let helper = server.add_test_comm_manager().unwrap();
    helper.add_ble_device("Massage Demo").await;
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
    // Check that we got an event back about a new device.
    let mut index = 0u32;
    while let Some(msg) = recv.next().await {
      if let ButtplugServerMessage::ScanningFinished(_) = msg {
        continue;
      } else if let ButtplugServerMessage::DeviceAdded(da) = msg {
        assert_eq!(da.device_name(), "Aneros Vivi");
        // Devices aren't guaranteed to be added in any specific order, the
        // scheduler will do whatever it wants. So check boundaries instead of
        // exact.
        assert!(da.device_index() < 2);
        index += 1;
        // Found both devices we're looking for, finish test.
        if index == 2 {
          break;
        }
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
fn test_server_scanning_finished() {
  async_manager::block_on(async {
    let server = ButtplugServer::default();
    let recv = server.event_stream();
    pin_mut!(recv);
    let helper = server.add_test_comm_manager().unwrap();
    helper.add_ble_device("Massage Demo").await;
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
    // Check that we got an event back about a new device.
    let mut count = 0u32;
    let mut finish_received = false;
    // We should get 3 messages: 2 DeviceAdded, 1 ScanningFinished.
    while let Some(msg) = recv.next().await {
      if matches!(msg, ButtplugServerMessage::ScanningFinished(_)) {
        finish_received = true;
        break;
      }
      count += 1;
      if count == 3 {
        break;
      }
    }
    assert!(finish_received);
    server
      .add_comm_manager::<util::DelayDeviceCommunicationManager>()
      .unwrap();
    helper.add_ble_device("Massage Demo").await;
    assert!(server
      .parse_message(messages::StartScanning::default().into())
      .await
      .is_ok());
  });
}


// TODO Test sending system message (Id 0)
// TODO Test sending system message (Ok but Id > 0)
// TODO Test repeated handshake
// TODO Test scan with no comm managers
// TODO Test message with no RequestServerInfo first
// TODO Test sending device command for device that doesn't exist (in server)