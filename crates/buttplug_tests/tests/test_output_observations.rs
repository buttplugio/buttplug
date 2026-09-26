// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;

use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
  message::{
    BUTTPLUG_CURRENT_API_MAJOR_VERSION,
    BUTTPLUG_CURRENT_API_MINOR_VERSION,
    ButtplugMessageSpecVersion,
    ButtplugServerMessageV4,
    OutputCmdV4,
    OutputCommand,
    OutputType,
    OutputValue,
    RequestServerInfoV4,
    StartScanningV0,
    StopCmdV4,
  },
};
use buttplug_server::message::{
  ButtplugClientMessageVariant,
  ButtplugServerMessageV3,
  ButtplugServerMessageVariant,
  RequestServerInfoV1,
  ScalarCmdV3,
  ScalarSubcommandV3,
  StopDeviceCmdV0,
  spec_enums::ButtplugCheckedClientMessageV4,
};
use futures::{FutureExt, Stream, StreamExt, pin_mut};
use std::{collections::BTreeSet, time::Duration};
use tokio::time::timeout;
use util::test_server_with_device_and_observations;

/// Waits for `count` distinct device indexes, failing instead of hanging on a stalled stream.
async fn wait_for_device_indexes(
  event_stream: &mut (impl Stream<Item = ButtplugServerMessageV4> + Unpin),
  count: usize,
) -> Vec<u32> {
  timeout(Duration::from_secs(5), async {
    let mut indexes = BTreeSet::new();
    while indexes.len() < count {
      match event_stream.next().await {
        Some(ButtplugServerMessageV4::DeviceList(dl)) => {
          indexes.extend(dl.devices().keys().copied());
        }
        Some(_) => {}
        None => panic!("event stream ended while waiting for device list"),
      }
    }
    indexes.into_iter().collect()
  })
  .await
  .expect("timed out waiting for device list")
}

#[tokio::test]
async fn test_ac2_1_observation_emission() {
  // AC2.1: Send a vibrate command at value 50, verify one OutputObservation
  // appears with correct device_index, feature_index, output_type="Vibrate", and value=50.0
  let (server, _device) = test_server_with_device_and_observations("Massage Demo");

  // Subscribe to observation stream
  let obs_stream = server
    .output_observation_stream()
    .expect("should be Some when enabled");
  pin_mut!(obs_stream);

  // Handshake
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .unwrap();

  // Start scanning and wait for device
  let event_stream = server.server_version_event_stream();
  pin_mut!(event_stream);
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StartScanningV0::default().into(),
    ))
    .await
    .unwrap();

  // Wait for DeviceList event to get device_index
  let device_index = loop {
    if let Some(ButtplugServerMessageV4::DeviceList(dl)) = event_stream.next().await
      && let Some((&idx, _)) = dl.devices().iter().next()
    {
      break idx;
    }
  };

  // Send vibrate command at value 50
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      OutputCmdV4::new(
        device_index,
        0,
        OutputCommand::Vibrate(OutputValue::new(50)),
      )
      .into(),
    ))
    .await
    .unwrap();

  // Verify observation appears with correct values
  if let Ok(Some(obs)) = timeout(Duration::from_millis(500), obs_stream.next()).await {
    assert_eq!(obs.device_index, device_index);
    assert_eq!(obs.feature_index, 0);
    assert_eq!(obs.output_type, "Vibrate");
    assert_eq!(obs.value, 50.0);
  } else {
    panic!("Expected observation but none received or timeout");
  }
}

#[tokio::test]
async fn test_ac2_2_observation_dedup() {
  // AC2.2: Send vibrate at value 50 twice. First should produce observation,
  // second should not. Verify by using tokio::time::timeout on the stream —
  // second read should time out.
  let (server, _device) = test_server_with_device_and_observations("Massage Demo");

  let obs_stream = server
    .output_observation_stream()
    .expect("should be Some when enabled");
  pin_mut!(obs_stream);

  // Handshake
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .unwrap();

  // Start scanning and wait for device
  let event_stream = server.server_version_event_stream();
  pin_mut!(event_stream);
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StartScanningV0::default().into(),
    ))
    .await
    .unwrap();

  let device_index = loop {
    if let Some(ButtplugServerMessageV4::DeviceList(dl)) = event_stream.next().await
      && let Some((&idx, _)) = dl.devices().iter().next()
    {
      break idx;
    }
  };

  // Send first vibrate command at value 50
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      OutputCmdV4::new(
        device_index,
        0,
        OutputCommand::Vibrate(OutputValue::new(50)),
      )
      .into(),
    ))
    .await
    .unwrap();

  // Verify first observation appears
  if let Ok(Some(obs)) = timeout(Duration::from_millis(500), obs_stream.next()).await {
    assert_eq!(obs.device_index, device_index);
    assert_eq!(obs.value, 50.0);
  } else {
    panic!("Expected first observation but none received or timeout");
  }

  // Send same vibrate command again at value 50
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      OutputCmdV4::new(
        device_index,
        0,
        OutputCommand::Vibrate(OutputValue::new(50)),
      )
      .into(),
    ))
    .await
    .unwrap();

  // Verify second observation does NOT appear (timeout expected)
  let result = timeout(Duration::from_millis(100), obs_stream.next()).await;
  assert!(
    result.is_err(),
    "Expected timeout (no observation) for deduplicated command"
  );
}

#[tokio::test]
async fn test_ac2_3_observation_before_protocol() {
  // AC2.3: Observations are emitted after the dedup check passes but before
  // protocol processing. This is verified structurally by the tap point location
  // (before handle_output_cmd). Test verifies observation arrives even when the
  // test device channel hasn't consumed the hardware command yet.
  let (server, _device) = test_server_with_device_and_observations("Massage Demo");

  let obs_stream = server
    .output_observation_stream()
    .expect("should be Some when enabled");
  pin_mut!(obs_stream);

  // Handshake
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .unwrap();

  // Start scanning and wait for device
  let event_stream = server.server_version_event_stream();
  pin_mut!(event_stream);
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StartScanningV0::default().into(),
    ))
    .await
    .unwrap();

  let device_index = loop {
    if let Some(ButtplugServerMessageV4::DeviceList(dl)) = event_stream.next().await
      && let Some((&idx, _)) = dl.devices().iter().next()
    {
      break idx;
    }
  };

  // Send vibrate command
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      OutputCmdV4::new(
        device_index,
        0,
        OutputCommand::Vibrate(OutputValue::new(75)),
      )
      .into(),
    ))
    .await
    .unwrap();

  // Verify observation appears (not waiting on device to process hardware command)
  if let Ok(Some(obs)) = timeout(Duration::from_millis(500), obs_stream.next()).await {
    assert_eq!(obs.device_index, device_index);
    assert_eq!(obs.output_type, "Vibrate");
    assert_eq!(obs.value, 75.0);
  } else {
    panic!("Expected observation but none received or timeout");
  }
}

#[tokio::test]
async fn test_ac3_1_stop_as_zero() {
  // AC3.1: Send vibrate at value 50, then send StopDeviceCmd for that device.
  // Verify zero-value observation appears after stop.
  let (server, _device) = test_server_with_device_and_observations("Massage Demo");

  let obs_stream = server
    .output_observation_stream()
    .expect("should be Some when enabled");
  pin_mut!(obs_stream);

  // Handshake
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .unwrap();

  // Start scanning and wait for device
  let event_stream = server.server_version_event_stream();
  pin_mut!(event_stream);
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StartScanningV0::default().into(),
    ))
    .await
    .unwrap();

  let device_index = loop {
    if let Some(ButtplugServerMessageV4::DeviceList(dl)) = event_stream.next().await
      && let Some((&idx, _)) = dl.devices().iter().next()
    {
      break idx;
    }
  };

  // Send vibrate command at value 50
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      OutputCmdV4::new(
        device_index,
        0,
        OutputCommand::Vibrate(OutputValue::new(50)),
      )
      .into(),
    ))
    .await
    .unwrap();

  // Verify first observation
  if let Ok(Some(obs)) = timeout(Duration::from_millis(500), obs_stream.next()).await {
    assert_eq!(obs.value, 50.0);
  } else {
    panic!("Expected first observation but none received or timeout");
  }

  // Send StopDeviceCmd for that specific device (device_index, None feature_index, outputs=true)
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StopCmdV4::new(Some(device_index), None, false, true).into(),
    ))
    .await
    .unwrap();

  // Verify zero-value observation appears
  if let Ok(Some(obs)) = timeout(Duration::from_millis(500), obs_stream.next()).await {
    assert_eq!(obs.device_index, device_index);
    assert_eq!(obs.feature_index, 0);
    assert_eq!(obs.output_type, "Vibrate");
    assert_eq!(obs.value, 0.0);
  } else {
    panic!("Expected zero-value observation after stop but none received or timeout");
  }
}

#[tokio::test]
async fn test_ac3_2_stop_all_devices() {
  // AC3.2: Send vibrate command, then send StopAllDevices.
  // Verify zero-value observation appears (stop all targets all devices).
  let (server, _device) = test_server_with_device_and_observations("Massage Demo");

  let obs_stream = server
    .output_observation_stream()
    .expect("should be Some when enabled");
  pin_mut!(obs_stream);

  // Handshake
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .unwrap();

  // Start scanning and wait for device
  let event_stream = server.server_version_event_stream();
  pin_mut!(event_stream);
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StartScanningV0::default().into(),
    ))
    .await
    .unwrap();

  let device_index = loop {
    if let Some(ButtplugServerMessageV4::DeviceList(dl)) = event_stream.next().await
      && let Some((&idx, _)) = dl.devices().iter().next()
    {
      break idx;
    }
  };

  // Send vibrate command
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      OutputCmdV4::new(
        device_index,
        0,
        OutputCommand::Vibrate(OutputValue::new(50)),
      )
      .into(),
    ))
    .await
    .unwrap();

  // Consume the emission observation
  let _ = timeout(Duration::from_millis(500), obs_stream.next()).await;

  // Send StopAllDevices (StopCmdV4::default() with no device_index)
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StopCmdV4::default().into(),
    ))
    .await
    .unwrap();

  // Verify zero-value observation appears
  if let Ok(Some(obs)) = timeout(Duration::from_millis(500), obs_stream.next()).await {
    assert_eq!(obs.device_index, device_index);
    assert_eq!(obs.feature_index, 0);
    assert_eq!(obs.output_type, "Vibrate");
    assert_eq!(obs.value, 0.0);
  } else {
    panic!("Expected zero-value observation after StopAllDevices but none received or timeout");
  }
}

#[tokio::test]
async fn test_ac3_3_stop_dedup() {
  // AC3.3: Stop-generated zero commands still go through the dedup path.
  // When a device is already at zero, sending another stop should not generate
  // an observation due to dedup (the zero-value command matches the previous state).
  let (server, _device) = test_server_with_device_and_observations("Massage Demo");

  let obs_stream = server
    .output_observation_stream()
    .expect("should be Some when enabled");
  pin_mut!(obs_stream);

  // Handshake
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .unwrap();

  // Start scanning and wait for device
  let event_stream = server.server_version_event_stream();
  pin_mut!(event_stream);
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StartScanningV0::default().into(),
    ))
    .await
    .unwrap();

  let device_index = loop {
    if let Some(ButtplugServerMessageV4::DeviceList(dl)) = event_stream.next().await
      && let Some((&idx, _)) = dl.devices().iter().next()
    {
      break idx;
    }
  };

  // Test 1: Send a non-zero command, verify observation appears
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      OutputCmdV4::new(
        device_index,
        0,
        OutputCommand::Vibrate(OutputValue::new(50)),
      )
      .into(),
    ))
    .await
    .unwrap();

  if let Ok(Some(obs)) = timeout(Duration::from_millis(500), obs_stream.next()).await {
    assert_eq!(obs.value, 50.0);
  } else {
    panic!("Expected observation for non-zero command");
  }

  // Test 2: Send StopDeviceCmd to set to zero, verify observation(s) appear
  // The device has multiple output features, so stop generates multiple observations
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StopCmdV4::new(Some(device_index), None, false, true).into(),
    ))
    .await
    .unwrap();

  // Consume all stop observations for this device (one per output feature)
  // The Aneros Vivi has 2 vibrate features, so we expect at least 2 observations
  let mut stop_obs_count = 0;
  for _ in 0..10 {
    // Allow up to 10 observations to account for any feature combinations
    match timeout(Duration::from_millis(100), obs_stream.next()).await {
      Ok(Some(obs)) => {
        assert_eq!(obs.value, 0.0);
        stop_obs_count += 1;
        // Keep consuming until we timeout
      }
      Err(_) => {
        // Timeout - no more observations
        break;
      }
      Ok(None) => {
        // Stream ended unexpectedly
        panic!("Observation stream ended unexpectedly during Test 2");
      }
    }
  }
  assert!(
    stop_obs_count >= 1,
    "Expected at least one zero-value observation for stop command"
  );

  // Test 3: Verify that the stop command successfully set the device to zero
  // by checking that a second stop doesn't generate an observation.
  // The dedup check should prevent sending a zero-value command twice.
  // AC3.3 requirement: "Stop dedup: no observation if already at zero."
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StopCmdV4::new(Some(device_index), None, false, true).into(),
    ))
    .await
    .unwrap();

  // According to AC3.3, the second stop command should NOT produce an observation
  // because the device is already at zero. The dedup logic in handle_outputcmd_v4
  // checks if the new message matches the last message and returns early if so,
  // preventing the observation from being sent.
  let result = timeout(Duration::from_millis(100), obs_stream.next()).await;
  assert!(
    result.is_err(),
    "Expected timeout (no observation) for deduplicated stop command when device is already at zero"
  );
}

#[tokio::test]
async fn test_stop_cmd_targets_single_device() {
  let (server, _devices) =
    util::test_servers_with_devices_and_observations(&["Massage Demo", "Massage Demo"]);

  let obs_stream = server
    .output_observation_stream()
    .expect("should be Some when enabled");
  pin_mut!(obs_stream);

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .unwrap();

  let event_stream = server.server_version_event_stream();
  pin_mut!(event_stream);
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StartScanningV0::default().into(),
    ))
    .await
    .unwrap();

  let mut device_indexes = wait_for_device_indexes(&mut event_stream, 2)
    .await
    .into_iter();
  let device_a_index = device_indexes.next().unwrap();
  let device_b_index = device_indexes.next().unwrap();

  // Vibrate both devices
  for index in [device_a_index, device_b_index] {
    server
      .parse_message(ButtplugClientMessageVariant::V4(
        OutputCmdV4::new(index, 0, OutputCommand::Vibrate(OutputValue::new(50))).into(),
      ))
      .await
      .unwrap();
    if let Ok(Some(obs)) = timeout(Duration::from_millis(500), obs_stream.next()).await {
      assert_eq!(obs.value, 50.0);
    } else {
      panic!("Expected vibrate observation for device {index}");
    }
  }

  // Stop only device_b
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StopCmdV4::new(Some(device_b_index), None, false, true).into(),
    ))
    .await
    .unwrap();

  // Stop observations are buffered before parse_message resolves, so now_or_never is deterministic.
  let mut saw_device_b_observation = false;
  while let Some(Some(obs)) = obs_stream.next().now_or_never() {
    assert_eq!(
      obs.device_index, device_b_index,
      "stop leaked to a device that was not targeted"
    );
    assert_eq!(obs.value, 0.0);
    saw_device_b_observation = true;
  }
  assert!(
    saw_device_b_observation,
    "Expected at least one zero-value observation for the stopped device"
  );
}

#[tokio::test]
async fn test_stop_cmd_unknown_device_returns_error() {
  // v4 validation rejects unknown indexes up front, so only the legacy path reaches stop_devices.
  let (server, _device) = util::test_server_with_device_and_observations("Massage Demo");

  server
    .parse_message(ButtplugClientMessageVariant::V3(
      RequestServerInfoV1::new("Test", ButtplugMessageSpecVersion::Version3).into(),
    ))
    .await
    .unwrap();

  let err = server
    .parse_message(ButtplugClientMessageVariant::V3(
      StopDeviceCmdV0::new(999).into(),
    ))
    .await
    .unwrap_err();

  if let ButtplugServerMessageVariant::V3(ButtplugServerMessageV3::Error(e)) = err {
    assert!(matches!(
      e.original_error(),
      ButtplugError::ButtplugDeviceError(ButtplugDeviceError::DeviceNotAvailable(999))
    ));
  } else {
    panic!("Expected a V3 error message, got {err:?}");
  }
}

#[tokio::test]
async fn test_legacy_stop_device_cmd_targets_single_device() {
  let (server, _devices) =
    util::test_servers_with_devices_and_observations(&["Massage Demo", "Massage Demo"]);

  let obs_stream = server
    .output_observation_stream()
    .expect("should be Some when enabled");
  pin_mut!(obs_stream);

  server
    .parse_message(ButtplugClientMessageVariant::V3(
      RequestServerInfoV1::new("Test", ButtplugMessageSpecVersion::Version3).into(),
    ))
    .await
    .unwrap();

  let event_stream = server.server_version_event_stream();
  pin_mut!(event_stream);
  server
    .parse_message(ButtplugClientMessageVariant::V3(
      StartScanningV0::default().into(),
    ))
    .await
    .unwrap();

  let mut device_indexes = wait_for_device_indexes(&mut event_stream, 2)
    .await
    .into_iter();
  let device_a_index = device_indexes.next().unwrap();
  let device_b_index = device_indexes.next().unwrap();

  for index in [device_a_index, device_b_index] {
    server
      .parse_message(ButtplugClientMessageVariant::V3(
        ScalarCmdV3::new(
          index,
          vec![ScalarSubcommandV3::new(0, 0.5, OutputType::Vibrate)],
        )
        .into(),
      ))
      .await
      .unwrap();
    if let Ok(Some(obs)) = timeout(Duration::from_millis(500), obs_stream.next()).await {
      assert!(obs.value > 0.0);
    } else {
      panic!("Expected vibrate observation for device {index}");
    }
  }

  // Legacy v3 StopDeviceCmd targeting only device_b.
  server
    .parse_message(ButtplugClientMessageVariant::V3(
      StopDeviceCmdV0::new(device_b_index).into(),
    ))
    .await
    .unwrap();

  let mut saw_device_b_observation = false;
  while let Some(Some(obs)) = obs_stream.next().now_or_never() {
    assert_eq!(
      obs.device_index, device_b_index,
      "legacy StopDeviceCmd leaked to a device that was not targeted"
    );
    assert_eq!(obs.value, 0.0);
    saw_device_b_observation = true;
  }
  assert!(
    saw_device_b_observation,
    "Expected at least one zero-value observation for the stopped device"
  );
}

#[tokio::test]
async fn test_stop_cmd_feature_scoped() {
  let (server, _device) = util::test_server_with_device_and_observations("Massage Demo");

  let obs_stream = server
    .output_observation_stream()
    .expect("should be Some when enabled");
  pin_mut!(obs_stream);

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .unwrap();

  let event_stream = server.server_version_event_stream();
  pin_mut!(event_stream);
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StartScanningV0::default().into(),
    ))
    .await
    .unwrap();

  let device_index = wait_for_device_indexes(&mut event_stream, 1).await[0];

  // Massage Demo (Aneros protocol) has two vibrate features, index 0 and 1.
  for feature_index in [0u32, 1u32] {
    server
      .parse_message(ButtplugClientMessageVariant::V4(
        OutputCmdV4::new(
          device_index,
          feature_index,
          OutputCommand::Vibrate(OutputValue::new(50)),
        )
        .into(),
      ))
      .await
      .unwrap();
    if let Ok(Some(obs)) = timeout(Duration::from_millis(500), obs_stream.next()).await {
      assert_eq!(obs.feature_index, feature_index);
      assert_eq!(obs.value, 50.0);
    } else {
      panic!("Expected vibrate observation for feature {feature_index}");
    }
  }

  // Stop only feature 0
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StopCmdV4::new(Some(device_index), Some(0), false, true).into(),
    ))
    .await
    .unwrap();

  if let Ok(Some(obs)) = timeout(Duration::from_millis(500), obs_stream.next()).await {
    assert_eq!(obs.feature_index, 0);
    assert_eq!(obs.value, 0.0);
  } else {
    panic!("Expected zero-value observation for stopped feature");
  }

  // Feature 1 should still be running.
  assert!(
    obs_stream.next().now_or_never().is_none(),
    "Expected no observation for feature 1, but feature-scoped stop leaked to it"
  );
}

#[tokio::test]
async fn test_stop_cmd_unknown_feature_returns_error() {
  let (server, _device) = util::test_server_with_device_and_observations("Massage Demo");

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .unwrap();

  let event_stream = server.server_version_event_stream();
  pin_mut!(event_stream);
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StartScanningV0::default().into(),
    ))
    .await
    .unwrap();

  let device_index = wait_for_device_indexes(&mut event_stream, 1).await[0];

  let err = server
    .parse_message(ButtplugClientMessageVariant::V4(
      StopCmdV4::new(Some(device_index), Some(999), false, true).into(),
    ))
    .await
    .unwrap_err();

  if let ButtplugServerMessageVariant::V4(ButtplugServerMessageV4::Error(e)) = err {
    assert!(matches!(
      e.original_error(),
      ButtplugError::ButtplugDeviceError(ButtplugDeviceError::DeviceFeatureIndexError(2, 999))
    ));
  } else {
    panic!("Expected a V4 error message, got {err:?}");
  }
}

#[tokio::test]
async fn test_stop_cmd_feature_index_without_device_index_rejected() {
  let (server, _device) = util::test_server_with_device_and_observations("Massage Demo");

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .unwrap();

  let err = server
    .parse_message(ButtplugClientMessageVariant::V4(
      StopCmdV4::new(None, Some(0), false, true).into(),
    ))
    .await
    .unwrap_err();

  if let ButtplugServerMessageVariant::V4(ButtplugServerMessageV4::Error(e)) = err {
    assert!(matches!(
      e.original_error(),
      ButtplugError::ButtplugMessageError(ButtplugMessageError::InvalidMessageContents(_))
    ));
  } else {
    panic!("Expected a V4 error message, got {err:?}");
  }
}

#[tokio::test]
async fn test_parse_checked_message_bypasses_spec_enums_unknown_feature() {
  // parse_checked_message skips spec_enums validation, so the device handle guard must catch this.
  let (server, _device) = util::test_server_with_device_and_observations("Massage Demo");

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .unwrap();

  let event_stream = server.server_version_event_stream();
  pin_mut!(event_stream);
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StartScanningV0::default().into(),
    ))
    .await
    .unwrap();

  let device_index = wait_for_device_indexes(&mut event_stream, 1).await[0];

  let err = server
    .parse_checked_message(ButtplugCheckedClientMessageV4::StopCmd(StopCmdV4::new(
      Some(device_index),
      Some(999),
      true,
      true,
    )))
    .await
    .unwrap_err();

  assert!(matches!(
    err.original_error(),
    ButtplugError::ButtplugDeviceError(ButtplugDeviceError::DeviceFeatureIndexError(2, 999))
  ));
}

#[tokio::test]
async fn test_parse_checked_message_bypasses_spec_enums_feature_without_device() {
  // parse_checked_message skips spec_enums validation, so stop_devices must reject this itself.
  let (server, _device) = util::test_server_with_device_and_observations("Massage Demo");

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .unwrap();

  let err = server
    .parse_checked_message(ButtplugCheckedClientMessageV4::StopCmd(StopCmdV4::new(
      None,
      Some(0),
      true,
      true,
    )))
    .await
    .unwrap_err();

  assert!(matches!(
    err.original_error(),
    ButtplugError::ButtplugMessageError(ButtplugMessageError::InvalidMessageContents(_))
  ));
}

#[tokio::test]
async fn test_ac5_1_disabled_no_observation_stream() {
  // AC5.1: When emit_output_observations is false, output_observation_stream()
  // returns None and there's no overhead.
  let (server, _device) = util::test_server_with_device("Massage Demo");

  // Verify observation stream is None when not enabled
  let obs_stream = server.output_observation_stream();
  assert!(
    obs_stream.is_none(),
    "output_observation_stream should be None when disabled"
  );
}
