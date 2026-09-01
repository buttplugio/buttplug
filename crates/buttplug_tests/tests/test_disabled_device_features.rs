// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;

use buttplug_client::ButtplugClientEvent;
use buttplug_core::message::{
  BUTTPLUG_CURRENT_API_MAJOR_VERSION, BUTTPLUG_CURRENT_API_MINOR_VERSION, ButtplugServerMessageV4,
  OutputCmdV4, OutputCommand, OutputHwPositionWithDuration, OutputType, OutputValue,
  RequestServerInfoV4, StartScanningV0, StopCmdV4,
};
use buttplug_server::message::{
  ButtplugClientMessageVariant, ButtplugServerMessageVariant, ScalarCmdV3, ScalarSubcommandV3,
};
use buttplug_server::{ButtplugServerBuilder, device::ServerDeviceManagerBuilder};
use buttplug_server_device_config::load_protocol_configs;
use futures::{StreamExt, pin_mut};
use std::time::Duration;
use tokio::time::timeout;
use util::{
  test_client_with_device_and_custom_dcm,
  test_device_manager::{TestDeviceCommunicationManagerBuilder, TestDeviceIdentifier},
};

const USER_CONFIG_DISABLED_HW_POSITION: &str = include_str!(
  "util/device_test/device_test_case/config/tcode_disabled_hw_position_user_config.json"
);

const USER_CONFIG_DISABLED_POSITION: &str =
  include_str!("util/device_test/device_test_case/config/tcode_disabled_position_user_config.json");

const USER_CONFIG_DISABLED_BOTH: &str = include_str!(
  "util/device_test/device_test_case/config/tcode_disabled_both_outputs_user_config.json"
);

fn load_dcm_with_config(config: &str) -> buttplug_server_device_config::DeviceConfigurationManager {
  load_protocol_configs(&None, &Some(config.to_string()), false)
    .expect("Test, assuming infallible.")
    .finish()
    .expect("Test, assuming infallible.")
}

fn test_identifier() -> TestDeviceIdentifier {
  TestDeviceIdentifier::new(
    "tcode-v03-disabled-test",
    Some("tcode-disabled-test-addr".into()),
  )
}

/// Helper: connect a client, scan, and return the first DeviceAdded event.
async fn get_client_device_from_config(config: &str) -> buttplug_client::ButtplugClientDevice {
  let dcm = load_dcm_with_config(config);
  let (client, _device_channel) =
    test_client_with_device_and_custom_dcm(&test_identifier(), dcm).await;

  let mut event_stream = client.event_stream();
  client
    .start_scanning()
    .await
    .expect("Test, assuming infallible.");

  while let Some(msg) = event_stream.next().await {
    if let ButtplugClientEvent::DeviceAdded(da) = msg {
      return da;
    }
  }
  panic!("No DeviceAdded event received");
}

/// Helper: set up a raw server, handshake, scan, and return the DeviceList with the device index.
async fn get_server_device_list(
  config: &str,
) -> (
  buttplug_server::ButtplugServer,
  u32,
  buttplug_core::message::v4::DeviceListV4,
  util::test_device_manager::TestDeviceChannelHost,
) {
  let dcm = load_dcm_with_config(config);
  let mut builder = TestDeviceCommunicationManagerBuilder::default();
  let device_channel = builder.add_test_device(&test_identifier());

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

  while let Some(msg) = recv.next().await {
    if let ButtplugServerMessageVariant::V4(ButtplugServerMessageV4::DeviceList(list)) = msg
      && !list.devices().is_empty()
    {
      let device_index = *list
        .devices()
        .keys()
        .next()
        .expect("Checked non-empty above");
      return (server, device_index, list, device_channel);
    }
  }
  panic!("No DeviceList received");
}

// ---------------------------------------------------------------------------
// Tests: Disabling HwPositionWithDuration
// ---------------------------------------------------------------------------

/// Verify that a disabled output type is absent from the DeviceList/DeviceAdded message the
/// client receives. After disabling hw_position_with_duration, the client should see only
/// position on feature 0.
#[tokio::test]
async fn test_disabled_hw_position_not_in_device_list() {
  let device = get_client_device_from_config(USER_CONFIG_DISABLED_HW_POSITION).await;
  assert!(
    device.output_available(OutputType::Position),
    "position should be available (not disabled)"
  );
  assert!(
    !device.output_available(OutputType::HwPositionWithDuration),
    "hw_position_with_duration should not be available (disabled in user config)"
  );
}

/// Verify the DeviceList message structure: feature 0 should contain exactly one output
/// (Position) and no HwPositionWithDuration when hw_position_with_duration is disabled.
#[tokio::test]
async fn test_disabled_hw_position_device_list_structure() {
  let (_server, _device_index, list, _device_channel) =
    get_server_device_list(USER_CONFIG_DISABLED_HW_POSITION).await;

  let device_info = list.devices().values().next().expect("One device expected");
  let feature = device_info
    .device_features()
    .get(&0)
    .expect("Feature 0 should exist");

  assert!(
    feature.contains_output(OutputType::Position),
    "Feature 0 should contain Position output"
  );
  assert!(
    !feature.contains_output(OutputType::HwPositionWithDuration),
    "Feature 0 should NOT contain HwPositionWithDuration output"
  );
}

/// Verify that the server rejects a command targeting a disabled output type, even if the client
/// constructs one directly. This guards against stale cached feature lists on older clients.
#[tokio::test]
async fn test_disabled_hw_position_command_rejected() {
  let (server, device_index, _list, _device_channel) =
    get_server_device_list(USER_CONFIG_DISABLED_HW_POSITION).await;

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

/// Verify the V3 scalar vector path also rejects commands targeting disabled output types.
#[tokio::test]
async fn test_disabled_hw_position_scalar_v3_command_rejected() {
  let (server, device_index, _list, _device_channel) =
    get_server_device_list(USER_CONFIG_DISABLED_HW_POSITION).await;

  let result = server
    .parse_message(ButtplugClientMessageVariant::V3(
      ScalarCmdV3::new(
        device_index,
        vec![ScalarSubcommandV3::new(
          0,
          0.5,
          OutputType::HwPositionWithDuration,
        )],
      )
      .into(),
    ))
    .await;

  assert!(
    result.is_err(),
    "Server should reject V3 scalar command targeting disabled output type hw_position_with_duration"
  );
}

/// Verify that Position commands are still accepted when only HwPositionWithDuration is disabled.
#[tokio::test]
async fn test_disabled_hw_position_allows_position_commands() {
  let (server, device_index, _list, _device_channel) =
    get_server_device_list(USER_CONFIG_DISABLED_HW_POSITION).await;

  let result = server
    .parse_message(ButtplugClientMessageVariant::V4(
      OutputCmdV4::new(
        device_index,
        0,
        OutputCommand::Position(OutputValue::new(500)),
      )
      .into(),
    ))
    .await;

  assert!(
    result.is_ok(),
    "Server should accept Position command when only HwPositionWithDuration is disabled"
  );
}

// ---------------------------------------------------------------------------
// Tests: Disabling Position (the other direction)
// ---------------------------------------------------------------------------

/// Verify that disabling Position leaves only HwPositionWithDuration in the device list.
#[tokio::test]
async fn test_disabled_position_not_in_device_list() {
  let device = get_client_device_from_config(USER_CONFIG_DISABLED_POSITION).await;
  assert!(
    !device.output_available(OutputType::Position),
    "position should not be available (disabled in user config)"
  );
  assert!(
    device.output_available(OutputType::HwPositionWithDuration),
    "hw_position_with_duration should be available (not disabled)"
  );
}

/// Verify the DeviceList structure when Position is disabled.
#[tokio::test]
async fn test_disabled_position_device_list_structure() {
  let (_server, _device_index, list, _device_channel) =
    get_server_device_list(USER_CONFIG_DISABLED_POSITION).await;

  let device_info = list.devices().values().next().expect("One device expected");
  let feature = device_info
    .device_features()
    .get(&0)
    .expect("Feature 0 should exist");

  assert!(
    !feature.contains_output(OutputType::Position),
    "Feature 0 should NOT contain Position output"
  );
  assert!(
    feature.contains_output(OutputType::HwPositionWithDuration),
    "Feature 0 should contain HwPositionWithDuration output"
  );
}

/// Verify that Position commands are rejected when Position is disabled.
#[tokio::test]
async fn test_disabled_position_command_rejected() {
  let (server, device_index, _list, _device_channel) =
    get_server_device_list(USER_CONFIG_DISABLED_POSITION).await;

  let result = server
    .parse_message(ButtplugClientMessageVariant::V4(
      OutputCmdV4::new(
        device_index,
        0,
        OutputCommand::Position(OutputValue::new(500)),
      )
      .into(),
    ))
    .await;

  assert!(
    result.is_err(),
    "Server should reject Position command when Position is disabled"
  );
}

/// Verify that HwPositionWithDuration commands are still accepted when only Position is disabled.
#[tokio::test]
async fn test_disabled_position_allows_hw_position_commands() {
  let (server, device_index, _list, _device_channel) =
    get_server_device_list(USER_CONFIG_DISABLED_POSITION).await;

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
    result.is_ok(),
    "Server should accept HwPositionWithDuration command when only Position is disabled"
  );
}

// ---------------------------------------------------------------------------
// Tests: Disabling both outputs on a feature
// ---------------------------------------------------------------------------

/// When all outputs on a feature are disabled (and no inputs exist), the feature should be
/// absent from the DeviceList entirely — the device_handle filter removes features that have
/// neither outputs nor inputs.
#[tokio::test]
async fn test_disabled_both_outputs_feature_absent() {
  let (_server, _device_index, list, _device_channel) =
    get_server_device_list(USER_CONFIG_DISABLED_BOTH).await;

  let device_info = list.devices().values().next().expect("One device expected");
  assert!(
    device_info.device_features().is_empty(),
    "Device should have no features when all outputs are disabled and no inputs exist"
  );
}

/// Stopping a device with only disabled outputs must not emit zero-value hardware commands.
#[tokio::test]
async fn test_disabled_both_outputs_stop_emits_no_commands() {
  let (server, device_index, _list, mut device_channel) =
    get_server_device_list(USER_CONFIG_DISABLED_BOTH).await;

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StopCmdV4::new(Some(device_index), None, false, true).into(),
    ))
    .await
    .expect("Stop should succeed even when all outputs are disabled");

  assert!(
    timeout(Duration::from_millis(150), device_channel.receiver.recv())
      .await
      .is_err(),
    "Stop must not emit hardware commands for disabled outputs"
  );
}

/// Verify at the client level that neither output type is available when both are disabled.
#[tokio::test]
async fn test_disabled_both_outputs_client_view() {
  let device = get_client_device_from_config(USER_CONFIG_DISABLED_BOTH).await;
  assert!(
    !device.output_available(OutputType::Position),
    "position should not be available (disabled)"
  );
  assert!(
    !device.output_available(OutputType::HwPositionWithDuration),
    "hw_position_with_duration should not be available (disabled)"
  );
}
