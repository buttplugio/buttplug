// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;
use buttplug::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    message::{
      ButtplugServerMessageV4,
      Endpoint,
      RequestServerInfoV1,
      StartScanningV0,
      BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION,
    },
  },
  server::message::{
    checked_raw_read_cmd::CheckedRawReadCmdV2, checked_raw_subscribe_cmd::CheckedRawSubscribeCmdV2, checked_raw_unsubscribe_cmd::CheckedRawUnsubscribeCmdV2, checked_raw_write_cmd::CheckedRawWriteCmdV2, spec_enums::ButtplugCheckedClientMessageV4, ButtplugClientMessageVariant, ButtplugServerMessageV3, ButtplugServerMessageVariant
  },
};

use futures::{pin_mut, StreamExt};
use tracing::info;
use std::matches;
pub use util::test_device_manager::TestDeviceCommunicationManagerBuilder;
use util::{setup_logging, test_server_v4_with_device, test_server_with_device};

// Test devices that have protocols that support movements not all devices do.
// For instance, the Onyx+ is part of a protocol that supports vibration, but
// the device itself does not.
#[tokio::test]
async fn test_capabilities_exposure() {
  tracing_subscriber::fmt::init();
  // Hold the channel but don't do anything with it.
  let (server, _channel) = test_server_with_device("Onyx+", false);
  let recv = server.event_stream();
  pin_mut!(recv);

  server
    .parse_message(ButtplugClientMessageVariant::V3(
      RequestServerInfoV1::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION).into(),
    ))
    .await
    .expect("Test, assuming infallible.");
  server
    .parse_message(ButtplugClientMessageVariant::V3(
      StartScanningV0::default().into(),
    ))
    .await
    .expect("Test, assuming infallible.");
  while let Some(msg) = recv.next().await {
    if let ButtplugServerMessageVariant::V3(ButtplugServerMessageV3::DeviceAdded(device)) = msg {
      assert!(device.device_messages().scalar_cmd().is_none());
      assert!(device.device_messages().linear_cmd().is_some());
      return;
    }
  }
}

#[ignore = "Needs to be fixed"]
#[tokio::test]
async fn test_server_raw_message() {
  let (server, _) = test_server_with_device("Massage Demo", true);
  let recv = server.event_stream();
  pin_mut!(recv);
  assert!(server
    .parse_message(ButtplugClientMessageVariant::V3(
      RequestServerInfoV1::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION).into()
    ))
    .await
    .is_ok());
  assert!(server
    .parse_message(ButtplugClientMessageVariant::V3(
      StartScanningV0::default().into()
    ))
    .await
    .is_ok());
  while let Some(msg) = recv.next().await {
    if let ButtplugServerMessageVariant::V3(ButtplugServerMessageV3::ScanningFinished(_)) = msg {
      continue;
    } else if let ButtplugServerMessageVariant::V3(ButtplugServerMessageV3::DeviceAdded(da)) = msg {
      assert!(da.device_messages().raw_read_cmd().is_some());
      assert!(da.device_messages().raw_write_cmd().is_some());
      assert!(da.device_messages().raw_subscribe_cmd().is_some());
      assert_eq!(da.device_name(), "Aneros Vivi (Raw Messages Allowed)");
      return;
    } else {
      panic!(
        "Returned message was not a DeviceAdded message or timed out: {:?}",
        msg
      );
    }
  }
}

#[tokio::test]
async fn test_server_no_raw_message() {
  let (server, _) = test_server_with_device("Massage Demo", false);
  let recv = server.event_stream();
  pin_mut!(recv);
  assert!(server
    .parse_message(ButtplugClientMessageVariant::V3(
      RequestServerInfoV1::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION).into()
    ))
    .await
    .is_ok());
  assert!(server
    .parse_message(ButtplugClientMessageVariant::V3(
      StartScanningV0::default().into()
    ))
    .await
    .is_ok());
  while let Some(msg) = recv.next().await {
    if let ButtplugServerMessageVariant::V3(ButtplugServerMessageV3::ScanningFinished(_)) = msg {
      continue;
    } else if let ButtplugServerMessageVariant::V3(ButtplugServerMessageV3::DeviceAdded(da)) = msg {
      assert_eq!(da.device_name(), "Aneros Vivi");
      assert!(da.device_messages().raw_read_cmd().is_none());
      assert!(da.device_messages().raw_write_cmd().is_none());
      assert!(da.device_messages().raw_subscribe_cmd().is_none());
      break;
    } else {
      panic!(
        "Returned message was not a DeviceAdded message or timed out: {:?}",
        msg
      );
    }
  }
}

#[ignore = "Needs to be fixed"]
#[tokio::test]
async fn test_reject_on_no_raw_message() {
  setup_logging();
  let (server, _) = test_server_v4_with_device("Massage Demo", false);
  let recv = server.server_version_event_stream();
  pin_mut!(recv);
  assert!(server
    .parse_checked_message(ButtplugCheckedClientMessageV4::from(
      RequestServerInfoV1::new("Test Client", BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION)
    ))
    .await
    .is_ok());
  assert!(server
    .parse_checked_message(ButtplugCheckedClientMessageV4::from(
      StartScanningV0::default()
    ))
    .await
    .is_ok());
  while let Some(msg) = recv.next().await {
    if let ButtplugServerMessageV4::ScanningFinished(_) = msg {
      continue;
    } else if let ButtplugServerMessageV4::DeviceAdded(da) = msg {
      assert_eq!(da.device_name(), "Aneros Vivi");
      let mut should_be_err;
      should_be_err = server
        .parse_checked_message(ButtplugCheckedClientMessageV4::from(CheckedRawWriteCmdV2::new(
          da.device_index(),
          Endpoint::Tx,
          &[0x0],
          false,
        )))
        .await;
      assert!(should_be_err.is_err());
      assert!(matches!(
        should_be_err.unwrap_err().original_error(),
        ButtplugError::ButtplugDeviceError(ButtplugDeviceError::MessageNotSupported(_))
      ));
      info!("ERRORED OUT");

      should_be_err = server
        .parse_checked_message(ButtplugCheckedClientMessageV4::from(CheckedRawReadCmdV2::new(
          da.device_index(),
          Endpoint::Tx,
          0,
          0,
        )))
        .await;
      assert!(should_be_err.is_err());
      assert!(matches!(
        should_be_err.unwrap_err().original_error(),
        ButtplugError::ButtplugDeviceError(ButtplugDeviceError::MessageNotSupported(_))
      ));

      should_be_err = server
        .parse_checked_message(ButtplugCheckedClientMessageV4::from(
          CheckedRawSubscribeCmdV2::new(da.device_index(), Endpoint::Tx),
        ))
        .await;
      assert!(should_be_err.is_err());
      assert!(matches!(
        should_be_err.unwrap_err().original_error(),
        ButtplugError::ButtplugDeviceError(ButtplugDeviceError::MessageNotSupported(_))
      ));

      should_be_err = server
        .parse_checked_message(ButtplugCheckedClientMessageV4::from(
          CheckedRawUnsubscribeCmdV2::new(da.device_index(), Endpoint::Tx),
        ))
        .await;
      assert!(should_be_err.is_err());
      assert!(matches!(
        should_be_err.unwrap_err().original_error(),
        ButtplugError::ButtplugDeviceError(ButtplugDeviceError::MessageNotSupported(_))
      ));
      return;
    } else {
      panic!(
        "Returned message was not a DeviceAdded message or timed out: {:?}",
        msg
      );
    }
  }
}

/*
#[cfg(target_os = "windows")]
#[ignore = "Has weird timeout issues"]
#[tokio::test]
async fn test_repeated_address_additions() {
    let mut server_builder = ButtplugServerBuilder::default();
    let builder = TestDeviceCommunicationManagerBuilder::default();
    let helper = builder.helper();
    server_builder.comm_manager(builder);
    let server = server_builder.finish().unwrap();
    let recv = server.event_stream();
    pin_mut!(recv);
    helper
      .add_ble_device_with_address("Massage Demo", "SameAddress")
      .await;
    helper
      .add_ble_device_with_address("Massage Demo", "SameAddress")
      .await;
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
    let mut device_index = None;
    let mut device_removed_called = true;
    while let Some(msg) = recv.next().await {
      match msg {
        ButtplugServerScanningFinished(_) => continue,
        ButtplugServerDeviceAdded(da) => {
          assert_eq!(da.device_name(), "Aneros Vivi");
          if device_index.is_none() {
            device_index = Some(da.device_index());
          } else {
            assert!(device_removed_called);
            assert_eq!(
              da.device_index(),
              device_index.expect("Test, assuming infallible.")
            );
            return;
          }
        }
        ButtplugServerDeviceRemoved(dr) => {
          assert_eq!(
            dr.device_index(),
            device_index.expect("Test, assuming infallible.")
          );
          device_removed_called = true;
        }
        _ => {
          panic!(
            "Returned message was not a DeviceAdded message or timed out: {:?}",
            msg
          );
        }
      }
    }
}
*/
