// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;

use buttplug_client::ButtplugClientEvent;
use buttplug_core::message::{
  BUTTPLUG_CURRENT_API_MAJOR_VERSION,
  BUTTPLUG_CURRENT_API_MINOR_VERSION,
  ButtplugServerMessageV4,
  OutputCmdV4,
  OutputCommand,
  OutputHwPositionWithDuration,
  OutputType,
  RequestServerInfoV4,
  StartScanningV0,
};
use buttplug_server::message::{ButtplugClientMessageVariant, ButtplugServerMessageVariant};
use buttplug_server::{ButtplugServerBuilder, device::ServerDeviceManagerBuilder};
use buttplug_server_device_config::load_protocol_configs;
use futures::{StreamExt, pin_mut};
use util::{
  test_client_with_device_and_custom_dcm,
  test_device_manager::{TestDeviceCommunicationManagerBuilder, TestDeviceIdentifier},
};

const USER_CONFIG: &str = include_str!(
  "util/device_test/device_test_case/config/tcode_disabled_hw_position_user_config.json"
);

fn load_disabled_test_dcm() -> buttplug_server_device_config::DeviceConfigurationManager {
  load_protocol_configs(&None, &Some(USER_CONFIG.to_string()), false)
    .expect("Test, assuming infallible.")
    .finish()
    .expect("Test, assuming infallible.")
}

/// Verify that a disabled output type is absent from the DeviceList/DeviceAdded message the
/// client receives. After disabling hw_position_with_duration, the client should see only
/// position on feature 0.
#[tokio::test]
async fn test_disabled_output_type_not_in_device_list() {
  let dcm = load_disabled_test_dcm();
  let identifier = TestDeviceIdentifier::new(
    "tcode-v03-disabled-test",
    Some("tcode-disabled-test-addr".into()),
  );

  let (client, _device_channel) = test_client_with_device_and_custom_dcm(&identifier, dcm).await;

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

  let device = client_device.expect("Test, assuming infallible.");
  assert!(
    device.output_available(OutputType::Position),
    "position should be available (not disabled)"
  );
  assert!(
    !device.output_available(OutputType::HwPositionWithDuration),
    "hw_position_with_duration should not be available (disabled in user config)"
  );
}

/// Verify that the server rejects a command targeting a disabled output type, even if the client
/// constructs one directly. This guards against stale cached feature lists on older clients.
#[tokio::test]
async fn test_disabled_output_type_command_rejected() {
  let dcm = load_disabled_test_dcm();
  let identifier = TestDeviceIdentifier::new(
    "tcode-v03-disabled-test",
    Some("tcode-disabled-test-addr".into()),
  );

  let mut builder = TestDeviceCommunicationManagerBuilder::default();
  let _device_channel = builder.add_test_device(&identifier);

  let mut dm_builder = ServerDeviceManagerBuilder::new(dcm);
  dm_builder.comm_manager(builder);

  let server = ButtplugServerBuilder::new(dm_builder.finish().unwrap())
    .finish()
    .unwrap();

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

  // Wait for the device to appear in a DeviceList update.
  let mut device_index = None;
  while let Some(msg) = recv.next().await {
    if let ButtplugServerMessageVariant::V4(ButtplugServerMessageV4::DeviceList(list)) = msg {
      if !list.devices().is_empty() {
        device_index = Some(
          *list
            .devices()
            .keys()
            .next()
            .expect("Checked non-empty above"),
        );
        break;
      }
    }
  }

  let device_index = device_index.expect("Test device should have appeared");

  // Directly construct a command targeting the disabled output type and verify the server rejects it.
  let result = server
    .parse_message(ButtplugClientMessageVariant::V4(
      OutputCmdV4::new(
        device_index,
        0,
        OutputCommand::HwPositionWithDuration(OutputHwPositionWithDuration::new(500, 1000)),
      )
      .into(),
    ))
    .await;

  assert!(
    result.is_err(),
    "Server should reject command targeting disabled output type hw_position_with_duration"
  );
}
