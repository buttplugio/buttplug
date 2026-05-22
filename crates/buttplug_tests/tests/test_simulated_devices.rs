// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;

use buttplug_core::message::{
  BUTTPLUG_CURRENT_API_MAJOR_VERSION,
  BUTTPLUG_CURRENT_API_MINOR_VERSION,
  ButtplugServerMessageV4,
  OutputCmdV4,
  OutputCommand,
  OutputValue,
  RequestServerInfoV4,
  StartScanningV0,
  StopCmdV4,
};
use buttplug_server::message::ButtplugClientMessageVariant;
use futures::{StreamExt, pin_mut};
use std::time::Duration;
use tokio::time::timeout;
use util::test_server_with_simulated_device;

#[tokio::test]
async fn test_simulated_1vibe_observation() {
  // AC7.1, AC7.2: Create a simulated-1vibe, send vibrate command at value 50,
  // verify observation with correct device_index, feature_index=0, output_type="Vibrate", value=50.0
  let server = test_server_with_simulated_device("simulated-1vibe", None);

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
async fn test_simulated_2vibe_observation() {
  // AC7.2: Multi-feature devices produce observations for each feature separately.
  // Use simulated-2vibe archetype which has 2 vibrate features (feature 0 and feature 1).
  // Send commands to each feature and verify different feature_index values in observations.
  let server = test_server_with_simulated_device("simulated-2vibe", None);

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

  // Send vibrate command to feature 0 at value 30
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      OutputCmdV4::new(
        device_index,
        0,
        OutputCommand::Vibrate(OutputValue::new(30)),
      )
      .into(),
    ))
    .await
    .unwrap();

  // Verify first observation for feature 0
  if let Ok(Some(obs)) = timeout(Duration::from_millis(500), obs_stream.next()).await {
    assert_eq!(obs.device_index, device_index);
    assert_eq!(obs.feature_index, 0);
    assert_eq!(obs.output_type, "Vibrate");
    assert_eq!(obs.value, 30.0);
  } else {
    panic!("Expected first observation but none received or timeout");
  }

  // Send vibrate command to feature 1 at value 70
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      OutputCmdV4::new(
        device_index,
        1,
        OutputCommand::Vibrate(OutputValue::new(70)),
      )
      .into(),
    ))
    .await
    .unwrap();

  // Verify second observation for feature 1
  if let Ok(Some(obs)) = timeout(Duration::from_millis(500), obs_stream.next()).await {
    assert_eq!(obs.device_index, device_index);
    assert_eq!(obs.feature_index, 1);
    assert_eq!(obs.output_type, "Vibrate");
    assert_eq!(obs.value, 70.0);
  } else {
    panic!("Expected second observation but none received or timeout");
  }
}

#[tokio::test]
async fn test_simulated_rotator_observation() {
  // AC7.2: Non-vibrate output types (Rotate, Oscillate, Position) produce observations
  // with the correct output_type string. Use simulated-rotator which has a Rotate feature.
  let server = test_server_with_simulated_device("simulated-rotator", None);

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

  // Send rotate command at value 50 to feature 0 (the rotator's rotate feature)
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      OutputCmdV4::new(device_index, 0, OutputCommand::Rotate(OutputValue::new(50))).into(),
    ))
    .await
    .unwrap();

  // Verify observation has correct output_type="Rotate"
  if let Ok(Some(obs)) = timeout(Duration::from_millis(500), obs_stream.next()).await {
    assert_eq!(obs.device_index, device_index);
    assert_eq!(obs.feature_index, 0);
    assert_eq!(obs.output_type, "Rotate");
    assert_eq!(obs.value, 50.0);
  } else {
    panic!("Expected observation but none received or timeout");
  }
}

#[tokio::test]
async fn test_simulated_diverse_archetypes() {
  // AC7.2: Verify various simulated archetypes can be created and discovered
  // (detailed feature testing happens with 1vibe tests)
  for archetype in &[
    "simulated-1vibe",
    "simulated-2vibe",
    "simulated-rotator",
    "simulated-oscillator",
    "simulated-stroker",
  ] {
    let server = test_server_with_simulated_device(archetype, None);

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

    // Verify the device appears
    loop {
      if let Some(ButtplugServerMessageV4::DeviceList(dl)) = event_stream.next().await {
        assert!(
          !dl.devices().is_empty(),
          "Archetype {} should be discovered",
          archetype
        );
        break;
      }
    }
  }
}

#[tokio::test]
async fn test_simulated_stop_produces_zero_observation() {
  // AC7.3: Create a simulated-1vibe, send vibrate at value 50,
  // then send Stop command, verify the stop produces a zero-value observation
  let server = test_server_with_simulated_device("simulated-1vibe", None);

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

  // Verify first observation
  if let Ok(Some(obs)) = timeout(Duration::from_millis(500), obs_stream.next()).await {
    assert_eq!(obs.value, 50.0);
  } else {
    panic!("Expected first observation but none received or timeout");
  }

  // Send StopDeviceCmd for that specific device
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
async fn test_simulated_device_appears_on_scan() {
  // AC1.3: Create a server with a simulated device, do handshake + scanning,
  // verify DeviceList contains the simulated device
  let server = test_server_with_simulated_device("simulated-1vibe", None);

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

  // Wait for DeviceList event - device should appear
  loop {
    if let Some(ButtplugServerMessageV4::DeviceList(dl)) = event_stream.next().await {
      assert!(
        !dl.devices().is_empty(),
        "Device should appear in DeviceList after scanning"
      );
      break;
    }
  }
}
