// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;
use buttplug::{
  client::{
    ButtplugClientDeviceEvent,
    ButtplugClientError,
    ButtplugClientEvent,
    ScalarValueCommand,
  },
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
    message::{
      self,
      ButtplugActuatorFeatureMessageType,
      ClientDeviceMessageAttributesV3,
      DeviceFeature,
      DeviceFeatureActuator,
      Endpoint,
      FeatureType,
    },
  },
  server::device::{
    configuration::{UserDeviceCustomization, UserDeviceDefinition, UserDeviceIdentifier},
    hardware::{HardwareCommand, HardwareWriteCmd},
  },
  util::{async_manager, device_configuration::load_protocol_configs},
};
use futures::StreamExt;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use util::test_device_manager::{check_test_recv_value, TestDeviceIdentifier};
use util::{
  test_client_with_device,
  test_client_with_device_and_custom_dcm,
  test_device_manager::TestHardwareEvent,
};

#[cfg(feature = "server")]
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

#[cfg(feature = "server")]
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

#[cfg(feature = "server")]
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

#[cfg(feature = "server")]
#[tokio::test]
async fn test_client_device_invalid_command() {
  tracing_subscriber::fmt::init();
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
      .vibrate(&ScalarValueCommand::ScalarValue(2.0))
      .await
      .unwrap_err(),
    ButtplugClientError::ButtplugError(ButtplugError::ButtplugMessageError(
      ButtplugMessageError::InvalidMessageContents(..)
    ))
  ));
  assert!(matches!(
    test_device
      .vibrate(&ScalarValueCommand::ScalarValueVec(vec!(0.5, 0.5, 0.5)))
      .await
      .unwrap_err(),
    ButtplugClientError::ButtplugError(ButtplugError::ButtplugDeviceError(
      ButtplugDeviceError::DeviceFeatureCountMismatch(..)
    ))
  ));
  assert!(matches!(
    test_device
      .vibrate(&ScalarValueCommand::ScalarValueVec(vec!()))
      .await
      .unwrap_err(),
    ButtplugClientError::ButtplugError(ButtplugError::ButtplugDeviceError(
      ButtplugDeviceError::ProtocolRequirementError(..)
    ))
  ));
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_client_repeated_deviceadded_message() {
  use buttplug::core::message::{
    ButtplugClientMessageV3,
    ButtplugClientMessageVariant,
    ButtplugServerMessageVariant,
  };

  let helper = Arc::new(util::channel_transport::ChannelClientTestHelper::new());
  helper.simulate_successful_connect().await;
  let helper_clone = helper.clone();
  let mut event_stream = helper.client().event_stream();
  async_manager::spawn(async move {
    assert!(matches!(
      helper_clone.next_client_message().await,
      ButtplugClientMessageVariant::V3(ButtplugClientMessageV3::StartScanning(..))
    ));
    helper_clone
      .send_client_incoming(ButtplugServerMessageVariant::V3(
        message::OkV0::new(3).into(),
      ))
      .await;
    let device_added = message::DeviceAddedV3::new(
      1,
      "Test Device",
      &None,
      &None,
      &ClientDeviceMessageAttributesV3::default(),
    );
    helper_clone
      .send_client_incoming(ButtplugServerMessageVariant::V3(
        device_added.clone().into(),
      ))
      .await;
    helper_clone
      .send_client_incoming(ButtplugServerMessageVariant::V3(device_added.into()))
      .await;
  });
  helper
    .client()
    .start_scanning()
    .await
    .expect("Test, assuming infallible.");
  assert!(matches!(
    event_stream
      .next()
      .await
      .expect("Test, assuming infallible."),
    ButtplugClientEvent::DeviceAdded(..)
  ));
  assert!(matches!(
    event_stream
      .next()
      .await
      .expect("Test, assuming infallible."),
    ButtplugClientEvent::Error(..)
  ));
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_client_repeated_deviceremoved_message() {
  use buttplug::core::message::{
    ButtplugClientMessageV3,
    ButtplugClientMessageVariant,
    ButtplugServerMessageVariant,
  };

  let helper = Arc::new(util::channel_transport::ChannelClientTestHelper::new());
  helper.simulate_successful_connect().await;
  let helper_clone = helper.clone();
  let mut event_stream = helper.client().event_stream();
  async_manager::spawn(async move {
    assert!(matches!(
      helper_clone.next_client_message().await,
      ButtplugClientMessageVariant::V3(ButtplugClientMessageV3::StartScanning(..))
    ));
    helper_clone
      .send_client_incoming(ButtplugServerMessageVariant::V3(
        message::OkV0::new(3).into(),
      ))
      .await;
    let device_added = message::DeviceAddedV3::new(
      1,
      "Test Device",
      &None,
      &None,
      &ClientDeviceMessageAttributesV3::default(),
    );
    let device_removed = message::DeviceRemovedV0::new(1);
    helper_clone
      .send_client_incoming(ButtplugServerMessageVariant::V3(device_added.into()))
      .await;
    helper_clone
      .send_client_incoming(ButtplugServerMessageVariant::V3(
        device_removed.clone().into(),
      ))
      .await;
    helper_clone
      .send_client_incoming(ButtplugServerMessageVariant::V3(device_removed.into()))
      .await;
  });
  helper
    .client()
    .start_scanning()
    .await
    .expect("Test, assuming infallible.");
  assert!(matches!(
    event_stream
      .next()
      .await
      .expect("Test, assuming infallible."),
    ButtplugClientEvent::DeviceAdded(..)
  ));
  assert!(matches!(
    event_stream
      .next()
      .await
      .expect("Test, assuming infallible."),
    ButtplugClientEvent::DeviceRemoved(..)
  ));
  assert!(matches!(
    event_stream
      .next()
      .await
      .expect("Test, assuming infallible."),
    ButtplugClientEvent::Error(..)
  ));
}

#[tokio::test]
async fn test_client_range_limits() {
  let dcm = load_protocol_configs(&None, &None, false)
    .expect("Test, assuming infallible.")
    .finish()
    .expect("Test, assuming infallible.");

  // Add a user config that configures the test device to only user the lower and upper half for the two vibrators
  let identifier = UserDeviceIdentifier::new("range-test", "aneros", &Some("Massage Demo".into()));
  let test_identifier = TestDeviceIdentifier::new("Massage Demo", Some("range-test".into()));
  dcm
    .add_user_device_definition(
      &identifier,
      &UserDeviceDefinition::new(
        "Massage Demo",
        &[
          DeviceFeature::new(
            "Lower half",
            FeatureType::Vibrate,
            &Some(DeviceFeatureActuator::new(
              &(0..=127),
              &(0..=64),
              &[ButtplugActuatorFeatureMessageType::ScalarCmd].into(),
            )),
            &None,
          ),
          DeviceFeature::new(
            "Upper half",
            FeatureType::Vibrate,
            &Some(DeviceFeatureActuator::new(
              &(0..=127),
              &(64..=127),
              &[ButtplugActuatorFeatureMessageType::ScalarCmd].into(),
            )),
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
      assert!(dev
        .vibrate(&ScalarValueCommand::ScalarValue(0.5))
        .await
        .is_ok());

      // Lower half
      check_test_recv_value(
        &mut device,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF1, 32], false)),
      );

      // Upper half
      check_test_recv_value(
        &mut device,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF2, 96], false)),
      );

      // Disable device
      assert!(dev
        .vibrate(&ScalarValueCommand::ScalarValue(0.0))
        .await
        .is_ok());

      // Lower half
      check_test_recv_value(
        &mut device,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF1, 0], false)),
      );

      // Upper half
      check_test_recv_value(
        &mut device,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF2, 0], false)),
      );
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
