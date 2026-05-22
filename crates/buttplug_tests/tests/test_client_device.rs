// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;
use buttplug_client::{ButtplugClientDeviceEvent, ButtplugClientError, ButtplugClientEvent};
use buttplug_core::util::{range::RangeInclusive, small_vec_enum_map::SmallVecEnumMap};
use buttplug_server::device::hardware::{HardwareCommand, HardwareWriteCmd};
use buttplug_server_device_config::{
  Endpoint, RangeWithLimit, ServerDeviceDefinitionBuilder, ServerDeviceFeature,
  ServerDeviceFeatureOutput, ServerDeviceFeatureOutputValueProperties, UserDeviceIdentifier,
  load_protocol_configs,
};
use futures::StreamExt;
use std::time::Duration;
use tokio::time::sleep;
use util::test_device_manager::{TestDeviceIdentifier, check_test_recv_value};
use util::{
  test_client_with_device, test_client_with_device_and_custom_dcm,
  test_device_manager::TestHardwareEvent,
};
use uuid::Uuid;

#[tokio::test]
async fn test_client_device_connected_status() {
  let (client, device) = test_client_with_device().await;

  let mut event_stream = client.event_stream();
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
    .sender
    .send(TestHardwareEvent::Disconnect)
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
}

#[tokio::test]
async fn test_client_device_client_disconnected_status() {
  let (client, _) = test_client_with_device().await;

  let mut event_stream = client.event_stream();
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
}

#[tokio::test]
async fn test_client_device_connected_no_event_listener() {
  let (client, device) = test_client_with_device().await;

  client
    .start_scanning()
    .await
    .expect("Test, assuming infallible.");
  sleep(Duration::from_millis(100)).await;
  device
    .sender
    .send(TestHardwareEvent::Disconnect)
    .await
    .expect("Test, assuming infallible.");
  sleep(Duration::from_millis(100)).await;
  client
    .disconnect()
    .await
    .expect("Test, assuming infallible.");
  assert!(!client.connected());
  sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_client_device_invalid_command() {
  let (client, _) = test_client_with_device().await;

  let mut event_stream = client.event_stream();
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
      .run_output(
        &buttplug_client::device::ClientDeviceOutputCommand::Vibrate(
          buttplug_client::device::ClientDeviceCommandValue::Steps(1000)
        )
      )
      .await
      .unwrap_err(),
    ButtplugClientError::ButtplugOutputCommandConversionError(_)
  ));
}

#[tokio::test]
async fn test_client_range_limits() {
  let dcm = load_protocol_configs(&None, &None, false)
    .expect("Test, assuming infallible.")
    .finish()
    .expect("Test, assuming infallible.");

  // Add a user config that maps the two vibrators to the lower and upper half
  // of the hardware range.
  let identifier = UserDeviceIdentifier::new("range-test", "aneros", &Some("Massage Demo".into()));
  let test_identifier = TestDeviceIdentifier::new("Massage Demo", Some("range-test".into()));
  let lower_output: SmallVecEnumMap<ServerDeviceFeatureOutput, 1> =
    vec![ServerDeviceFeatureOutput::Vibrate(
      ServerDeviceFeatureOutputValueProperties::new(
        RangeWithLimit::new_with_user(
          RangeInclusive::new(0, 127),
          Some(RangeInclusive::new(0, 64)),
        ),
        false,
      ),
    )]
    .into();
  let upper_output: SmallVecEnumMap<ServerDeviceFeatureOutput, 1> =
    vec![ServerDeviceFeatureOutput::Vibrate(
      ServerDeviceFeatureOutputValueProperties::new(
        RangeWithLimit::new_with_user(
          RangeInclusive::new(0, 127),
          Some(RangeInclusive::new(64, 127)),
        ),
        false,
      ),
    )]
    .into();
  let lower_feature = ServerDeviceFeature::new(
    0,
    "Lower half".to_owned(),
    Uuid::new_v4(),
    None,
    None,
    lower_output,
    SmallVecEnumMap::default(),
  );
  let upper_feature = ServerDeviceFeature::new(
    1,
    "Upper half".to_owned(),
    Uuid::new_v4(),
    None,
    None,
    upper_output,
    SmallVecEnumMap::default(),
  );
  let definition = ServerDeviceDefinitionBuilder::new("Massage Demo", &Uuid::new_v4())
    .add_feature(&lower_feature)
    .add_feature(&upper_feature)
    .finish();
  dcm.add_user_device_definition(&identifier, &definition);

  // Start the server & client
  let (client, mut device) = test_client_with_device_and_custom_dcm(&test_identifier, dcm).await;
  let mut event_stream = client.event_stream();
  assert!(client.start_scanning().await.is_ok());

  while let Some(event) = event_stream.next().await {
    if let ButtplugClientEvent::DeviceAdded(dev) = event {
      // Vibrate at half strength
      assert!(
        dev
          .run_output(
            &buttplug_client::device::ClientDeviceOutputCommand::Vibrate(
              buttplug_client::device::ClientDeviceCommandValue::Percent(0.5)
            )
          )
          .await
          .is_ok()
      );

      // Lower half
      check_test_recv_value(
        &Duration::from_millis(150),
        &mut device,
        HardwareCommand::Write(HardwareWriteCmd::new(
          &[Uuid::nil()],
          Endpoint::Tx,
          vec![0xF1, 32],
          false,
        )),
      )
      .await;

      // Upper half
      check_test_recv_value(
        &Duration::from_millis(150),
        &mut device,
        HardwareCommand::Write(HardwareWriteCmd::new(
          &[Uuid::nil()],
          Endpoint::Tx,
          vec![0xF2, 96],
          false,
        )),
      )
      .await;

      // Disable device
      assert!(
        dev
          .run_output(
            &buttplug_client::device::ClientDeviceOutputCommand::Vibrate(
              buttplug_client::device::ClientDeviceCommandValue::Steps(0)
            )
          )
          .await
          .is_ok()
      );

      // Lower half
      check_test_recv_value(
        &Duration::from_millis(150),
        &mut device,
        HardwareCommand::Write(HardwareWriteCmd::new(
          &[Uuid::nil()],
          Endpoint::Tx,
          vec![0xF1, 0],
          false,
        )),
      )
      .await;

      // Upper half
      check_test_recv_value(
        &Duration::from_millis(150),
        &mut device,
        HardwareCommand::Write(HardwareWriteCmd::new(
          &[Uuid::nil()],
          Endpoint::Tx,
          vec![0xF2, 0],
          false,
        )),
      )
      .await;
      break;
    }
  }
  assert!(client.stop_all_devices().await.is_ok());
}

// TODO Test invalid messages to device
// TODO Test invalid parameters in message
// TODO Test device invalidation across client connections (i.e. a device shouldn't be allowed to reconnect even if index is the same)
// TODO Test DeviceList being sent followed by repeat DeviceAdded
// TODO Test DeviceList being sent multiple times
// TODO Test sending device return for device that doesn't exist (in client)
