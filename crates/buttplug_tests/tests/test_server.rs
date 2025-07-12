// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;
use buttplug_server_device_config::Endpoint;
use util::test_server;
pub use util::{
  create_test_dcm,
  test_device_manager::{
    check_test_recv_value,
    TestDeviceCommunicationManagerBuilder,
    TestDeviceIdentifier,
  },
  test_server_with_comm_manager,
  test_server_with_device,
};

use buttplug_core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugHandshakeError},
    message::{
      OutputCmdV4,
      OutputCommand,
      OutputValue,
      ButtplugClientMessageV4,
      ButtplugMessageSpecVersion,
      ButtplugServerMessageV4,
      ErrorCode,
      PingV0,
      RequestServerInfoV4,
      ServerInfoV4,
      StartScanningV0,
      BUTTPLUG_CURRENT_API_MAJOR_VERSION,
      BUTTPLUG_CURRENT_API_MINOR_VERSION,
    },
  };
use buttplug_server::{
    device::{
      hardware::{HardwareCommand, HardwareWriteCmd},
      ServerDeviceManagerBuilder,
    },
    message::{
      checked_output_cmd::CheckedOutputCmdV4,
      spec_enums::ButtplugCheckedClientMessageV4,
      ButtplugClientMessageV3,
      ButtplugClientMessageVariant,
      ButtplugServerMessageV2,
      ButtplugServerMessageV3,
      ButtplugServerMessageVariant,
      RequestServerInfoV1,
      ServerInfoV2,
    },
    ButtplugServer,
    ButtplugServerBuilder,
};
use futures::{pin_mut, Stream, StreamExt};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

async fn setup_test_server(
  msg_union: ButtplugClientMessageV4,
) -> (
  ButtplugServer,
  impl Stream<Item = ButtplugServerMessageVariant>,
) {
  let server = test_server();
  let recv = server.event_stream();
  // assert_eq!(server.server_name, "Test Server");
  match server
    .parse_message(msg_union.into())
    .await
    .expect("Test, assuming infallible.")
  {
    ButtplugServerMessageVariant::V4(ButtplugServerMessageV4::ServerInfo(s)) => assert_eq!(
      s,
      ServerInfoV4::new(
        "Buttplug Server",
        ButtplugMessageSpecVersion::Version4,
        0,
        0
      )
    ),
    _ => panic!("Should've received ok"),
  }
  (server, recv)
}

#[tokio::test]
async fn test_server_handshake() {
  let msg = RequestServerInfoV4::new(
    "Test Client",
    BUTTPLUG_CURRENT_API_MAJOR_VERSION,
    BUTTPLUG_CURRENT_API_MINOR_VERSION,
  )
  .into();
  let (server, _recv) = setup_test_server(msg).await;
  assert!(server.connected());
}

#[tokio::test]
async fn test_server_handshake_not_done_first_v4() {
  let msg = ButtplugCheckedClientMessageV4::Ping(PingV0::default().into());
  let server = test_server();
  // assert_eq!(server.server_name, "Test Server");
  let result = server.parse_checked_message(msg).await;
  assert!(result.is_err());
  assert!(matches!(
    result.unwrap_err().original_error(),
    ButtplugError::ButtplugHandshakeError(ButtplugHandshakeError::RequestServerInfoExpected)
  ));
  assert!(!server.connected());
}

#[tokio::test]
async fn test_server_handshake_not_done_first_v3() {
  let msg = ButtplugClientMessageV3::Ping(PingV0::default().into());
  let server = test_server();
  // assert_eq!(server.server_name, "Test Server");
  let result = server.parse_message(msg.try_into().unwrap()).await;
  assert!(result.is_err());
  if let Err(ButtplugServerMessageVariant::V3(ButtplugServerMessageV3::Error(e))) = result {
    assert!(matches!(
      e.original_error(),
      ButtplugError::ButtplugHandshakeError(ButtplugHandshakeError::RequestServerInfoExpected)
    ));
  } else {
    panic!("Should've gotten error")
  }
  assert!(!server.connected());
}

#[tokio::test]
async fn test_client_version_older_than_server() {
  let msg = ButtplugClientMessageVariant::V2(
    RequestServerInfoV1::new("Test Client", ButtplugMessageSpecVersion::Version2).into(),
  );
  let server = test_server();
  // assert_eq!(server.server_name, "Test Server");
  match server
    .parse_message(msg)
    .await
    .expect("Test, assuming infallible.")
  {
    ButtplugServerMessageVariant::V2(ButtplugServerMessageV2::ServerInfo(s)) => assert_eq!(
      s,
      ServerInfoV2::new("Buttplug Server", ButtplugMessageSpecVersion::Version2, 0)
    ),
    _ => panic!("Should've received ok"),
  }
}

#[tokio::test]
#[ignore = "Needs to be rewritten to send in via the JSON parser, otherwise we're type bound due to the enum and can't fail"]
async fn test_server_version_older_than_client() {
  let server = test_server();
  let msg = ButtplugClientMessageVariant::V2(
    RequestServerInfoV1::new("Test Client", ButtplugMessageSpecVersion::Version2).into(),
  );
  assert!(
    server.parse_message(msg).await.is_err(),
    "Client having higher version than server should fail"
  );
}

#[tokio::test]
async fn test_ping_timeout() {
  let server = ButtplugServerBuilder::default()
    .max_ping_time(100)
    .finish()
    .expect("Test, assuming infallible.");
  let recv = server.event_stream();
  pin_mut!(recv);
  let msg = RequestServerInfoV4::new(
    "Test Client",
    BUTTPLUG_CURRENT_API_MAJOR_VERSION,
    BUTTPLUG_CURRENT_API_MINOR_VERSION,
  );
  sleep(Duration::from_millis(150)).await;
  let reply = server
    .parse_checked_message(ButtplugCheckedClientMessageV4::RequestServerInfo(msg))
    .await;
  assert!(
    reply.is_ok(),
    "ping timer shouldn't start until handshake finished. {:?}",
    reply
  );
  sleep(Duration::from_millis(300)).await;
  let pingmsg = PingV0::default();
  let result = server
    .parse_checked_message(ButtplugCheckedClientMessageV4::Ping(pingmsg.into()))
    .await;
  let err = result.unwrap_err();
  if !matches!(err.original_error(), ButtplugError::ButtplugPingError(_)) {
    panic!("Got wrong type of error back!");
  }
  // Check that we got an event back about the ping out.
  let msg = recv.next().await.expect("Test, assuming infallible.");
  if let ButtplugServerMessageVariant::V4(ButtplugServerMessageV4::Error(e)) = msg {
    if ErrorCode::ErrorPing != e.error_code() {
      panic!("Didn't get a ping error");
    }
  } else {
    panic!("Didn't get an error message back");
  }
}

#[tokio::test]
async fn test_device_stop_on_ping_timeout() {
  let mut builder = TestDeviceCommunicationManagerBuilder::default();
  let mut device = builder.add_test_device(&TestDeviceIdentifier::new("Massage Demo", None));

  let dm_builder = ServerDeviceManagerBuilder::new(create_test_dcm())
    .comm_manager(builder)
    .finish()
    .unwrap();

  let mut server_builder = ButtplugServerBuilder::new(dm_builder);
  server_builder.max_ping_time(100);
  let server = server_builder.finish().unwrap();

  let recv = server.server_version_event_stream();
  pin_mut!(recv);

  let msg = RequestServerInfoV4::new(
    "Test Client",
    BUTTPLUG_CURRENT_API_MAJOR_VERSION,
    BUTTPLUG_CURRENT_API_MINOR_VERSION,
  );
  let mut reply = server
    .parse_checked_message(ButtplugCheckedClientMessageV4::from(msg))
    .await;
  assert!(reply.is_ok());
  reply = server
    .parse_checked_message(ButtplugCheckedClientMessageV4::from(
      StartScanningV0::default(),
    ))
    .await;
  assert!(reply.is_ok());
  // Check that we got an event back about a new device.
  let mut device_index = 100;
  while let Some(msg) = recv.next().await {
    if let ButtplugServerMessageV4::ScanningFinished(_) = msg {
      continue;
    } else if let ButtplugServerMessageV4::DeviceList(list) = msg {
      let da = &list.devices()[&0];
      assert_eq!(da.device_name(), "Aneros Vivi");
      device_index = da.device_index();
      break;
    } else {
      panic!(
        "Returned message was not a DeviceAdded message or timed out: {:?}",
        msg
      );
    }
  }

  server
    .parse_checked_message(ButtplugCheckedClientMessageV4::from(
      CheckedOutputCmdV4::new(
        0,
        device_index,
        0,
        "f50a528b-b023-40f0-9906-df037443950a".try_into().unwrap(),
        OutputCommand::Vibrate(OutputValue::new(64)),
      ),
    ))
    .await
    .expect("Test, assuming infallible.");

  check_test_recv_value(
    &Duration::from_millis(150),
    &mut device,
    HardwareCommand::Write(HardwareWriteCmd::new(
      &[Uuid::nil()],
      Endpoint::Tx,
      vec![0xF1, 64],
      false,
    )),
  )
  .await;
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
    HardwareCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0xF1, 0], false)),
  );
   */
}

#[tokio::test]
async fn test_repeated_handshake() {
  let msg = RequestServerInfoV4::new(
    "Test Client",
    BUTTPLUG_CURRENT_API_MAJOR_VERSION,
    BUTTPLUG_CURRENT_API_MINOR_VERSION,
  );

  let (server, _recv) = setup_test_server((msg.clone()).into()).await;
  assert!(server.connected());
  let err = server
    .parse_message(ButtplugClientMessageVariant::V4(msg.into()))
    .await
    .unwrap_err();
  if let ButtplugServerMessageVariant::V4(ButtplugServerMessageV4::Error(e)) = err {
    assert!(matches!(
      e.original_error(),
      ButtplugError::ButtplugHandshakeError(ButtplugHandshakeError::HandshakeAlreadyHappened)
    ));
  } else {
    panic!("Should've gotten error")
  }
}

#[tokio::test]
async fn test_invalid_device_index() {
  let msg = RequestServerInfoV4::new(
    "Test Client",
    BUTTPLUG_CURRENT_API_MAJOR_VERSION,
    BUTTPLUG_CURRENT_API_MINOR_VERSION,
  );
  let (server, _) = setup_test_server(msg.into()).await;
  let err = server
    .parse_message(ButtplugClientMessageVariant::V4(
      OutputCmdV4::new(10, 0, OutputCommand::Vibrate(OutputValue::new(0))).into(),
    ))
    .await
    .unwrap_err();
  if let ButtplugServerMessageVariant::V4(ButtplugServerMessageV4::Error(e)) = err {
    assert!(matches!(
      e.original_error(),
      ButtplugError::ButtplugDeviceError(ButtplugDeviceError::DeviceNotAvailable(_))
    ));
  } else {
    panic!("Should've gotten error")
  }
}

#[tokio::test]
async fn test_device_index_generation() {
  let mut builder = TestDeviceCommunicationManagerBuilder::default();
  let mut _device1 = builder.add_test_device(&TestDeviceIdentifier::new("Massage Demo", None));
  let mut _device2 = builder.add_test_device(&TestDeviceIdentifier::new("Massage Demo", None));

  let server = test_server_with_comm_manager(builder);

  let recv = server.server_version_event_stream();
  pin_mut!(recv);
  assert!(server
    .parse_checked_message(
      RequestServerInfoV4::new(
        "Test Client",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION
      )
      .into()
    )
    .await
    .is_ok());
  assert!(server
    .parse_checked_message(StartScanningV0::default().into())
    .await
    .is_ok());
  // Check that we got an event back about a new device.
  let mut index = 0u32;
  while let Some(msg) = recv.next().await {
    if let ButtplugServerMessageV4::ScanningFinished(_) = msg {
      continue;
    } else if let ButtplugServerMessageV4::DeviceList(list) = msg {
      let da = &list.devices()[&0];
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
      panic!(
        "Returned message was not a DeviceAdded message or timed out: {:?}",
        msg
      );
    }
  }
}

#[tokio::test]
async fn test_server_scanning_finished() {
  let mut builder = TestDeviceCommunicationManagerBuilder::default();
  let mut _device1 = builder.add_test_device(&TestDeviceIdentifier::new("Massage Demo", None));
  let mut _device2 = builder.add_test_device(&TestDeviceIdentifier::new("Massage Demo", None));

  let server = test_server_with_comm_manager(builder);

  let recv = server.server_version_event_stream();
  pin_mut!(recv);
  assert!(server
    .parse_checked_message(
      RequestServerInfoV4::new(
        "Test Client",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION
      )
      .into()
    )
    .await
    .is_ok());
  assert!(server
    .parse_checked_message(StartScanningV0::default().into())
    .await
    .is_ok());
  // Check that we got an event back about a new device.
  let mut count = 0u32;
  let mut finish_received = false;
  // We should get 3 messages: 2 DeviceAdded, 1 ScanningFinished.
  while let Some(msg) = recv.next().await {
    if matches!(msg, ButtplugServerMessageV4::ScanningFinished(_)) {
      finish_received = true;
      break;
    }
    count += 1;
    if count == 3 {
      break;
    }
  }
  assert!(finish_received);
}

// TODO Test sending system message (Id 0)
// TODO Test sending system message (Ok but Id > 0)
// TODO Test scan with no comm managers
// TODO Test message with no RequestServerInfo first
// TODO Test sending device command for device that doesn't exist (in server)
