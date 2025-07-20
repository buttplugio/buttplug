// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;
use buttplug_client::{ButtplugClientDeviceEvent, ButtplugClientError, ButtplugClientEvent};
use buttplug_core::{
    errors::ButtplugError,
    message::{OutputType, FeatureType},
    util::async_manager
};
use buttplug_server_device_config::{load_protocol_configs, UserDeviceCustomization, DeviceDefinition, UserDeviceIdentifier, ServerDeviceFeature, ServerDeviceFeatureOutput, Endpoint};
use buttplug_server::{
    device::{
      hardware::{HardwareCommand, HardwareWriteCmd},
    },
};
use futures::StreamExt;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::sleep;
use util::test_device_manager::{check_test_recv_value, TestDeviceIdentifier};
use util::{
  test_client_with_device,
  test_client_with_device_and_custom_dcm,
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
  use buttplug_core::errors::ButtplugDeviceError;
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
    test_device.vibrate(1000).await.unwrap_err(),
    ButtplugClientError::ButtplugOutputCommandConversionError(_))
  );
}

/*
#[tokio::test]
async fn test_client_range_limits() {
  let dcm = load_protocol_configs(&None, &None, false)
    .expect("Test, assuming infallible.")
    .finish()
    .expect("Test, assuming infallible.");

  // Add a user config that configures the test device to only user the lower and upper half for the two vibrators
  let identifier = UserDeviceIdentifier::new("range-test", "aneros", &Some("Massage Demo".into()));
  let test_identifier = TestDeviceIdentifier::new("Massage Demo", Some("range-test".into()));
  let mut feature_1_actuator = HashMap::new();
  feature_1_actuator.insert(
    OutputType::Vibrate,
    ServerDeviceFeatureOutput::new(&(0..=127), &(0..=64)),
  );
  let mut feature_2_actuator = HashMap::new();
  feature_2_actuator.insert(
    OutputType::Vibrate,
    ServerDeviceFeatureOutput::new(&(0..=127), &(64..=127)),
  );
  dcm
    .add_user_device_definition(
      &identifier,
      &DeviceDefinition::new(
        "Massage Demo",
        &Uuid::new_v4(),
        &None,
        &None,
        &[
          ServerDeviceFeature::new(
            "Lower half",
            &Uuid::new_v4(),
            &None,
            FeatureType::Vibrate,
            &Some(feature_1_actuator),
            &None,
            &None,
          ),
          ServerDeviceFeature::new(
            "Upper half",
            &Uuid::new_v4(),
            &None,
            FeatureType::Vibrate,
            &Some(feature_2_actuator),
            &None,
            &None,
          ),
        ],
        &UserDeviceCustomization::default(),
      ),
    )
    .unwrap();

  // Start the server & client
  let (client, mut device) = test_client_with_device_and_custom_dcm(&test_identifier, dcm).await;
  let mut event_stream = client.event_stream();
  assert!(client.start_scanning().await.is_ok());

  while let Some(event) = event_stream.next().await {
    if let ButtplugClientEvent::DeviceAdded(dev) = event {
      // Vibrate at half strength
      assert!(dev.vibrate(32).await.is_ok());

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
      assert!(dev.vibrate(0).await.is_ok());

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

 */
// TODO Test invalid messages to device
// TODO Test invalid parameters in message
// TODO Test device invalidation across client connections (i.e. a device shouldn't be allowed to reconnect even if index is the same)
// TODO Test DeviceList being sent followed by repeat DeviceAdded
// TODO Test DeviceList being sent multiple times
// TODO Test sending device return for device that doesn't exist (in client)
