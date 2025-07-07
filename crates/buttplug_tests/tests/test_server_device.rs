// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;
use buttplug_core::{
    message::{
      ButtplugServerMessageV4,
      RequestServerInfoV4,
      StartScanningV0,
      BUTTPLUG_CURRENT_API_MAJOR_VERSION,
      BUTTPLUG_CURRENT_API_MINOR_VERSION,
    },
  };
use buttplug_server::message::{
    ButtplugClientMessageVariant,
    ButtplugServerMessageVariant,
};

use futures::{pin_mut, StreamExt};
pub use util::test_device_manager::TestDeviceCommunicationManagerBuilder;
use util::test_server_with_device;

// Test devices that have protocols that support movements not all devices do.
// For instance, the Onyx+ is part of a protocol that supports vibration, but
// the device itself does not.
#[tokio::test]
#[ignore = "Need to figure out what exposure we're testing here"]
async fn test_capabilities_exposure() {
  tracing_subscriber::fmt::init();
  // Hold the channel but don't do anything with it.
  let (server, _channel) = test_server_with_device("Onyx+");
  let recv = server.event_stream();
  pin_mut!(recv);

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Test Client",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .expect("Test, assuming infallible.");
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StartScanningV0::default().into(),
    ))
    .await
    .expect("Test, assuming infallible.");
  while let Some(msg) = recv.next().await {
    if let ButtplugServerMessageVariant::V4(ButtplugServerMessageV4::DeviceList(_list)) = msg {
      // TODO Figure out what we're actually testing here?!
      //assert!(device.device_features().iter().any(|x| x.actuator().));
      //assert!(device.device_messages().linear_cmd().is_some());
      return;
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
