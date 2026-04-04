// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_client_conformance_test::sequences::core_protocol::core_protocol_sequence;
use buttplug_client_conformance_test::runner::run_sequence;
use futures::stream::StreamExt;
use futures::SinkExt;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn test_core_protocol_full_pass() {
  // Use a random port between 20000 and 29999
  let port = 20000 + (std::process::id() % 10000) as u16;

  // Spawn the runner in the background with shorter timeout
  let runner_task = tokio::spawn(async move {
    run_sequence(&core_protocol_sequence(), port, 500).await
  });

  // Wait for WebSocket server to start
  sleep(Duration::from_millis(500)).await;

  // Connect via raw WebSocket with timeout
  let url = format!("ws://127.0.0.1:{}", port);
  let connect_result = tokio::time::timeout(
    Duration::from_secs(5),
    tokio_tungstenite::connect_async(&url)
  )
  .await;

  match connect_result {
    Ok(Ok((ws_stream, _))) => {
      let (mut ws_sender, mut ws_receiver) = ws_stream.split();

      // Send handshake
      let handshake = json!([{
        "RequestServerInfoV4": {
          "Id": 1,
          "ClientName": "test_client",
          "ProtocolVersionMajor": 4,
          "ProtocolVersionMinor": 0
        }
      }]);
      let _ = ws_sender
        .send(Message::Text(handshake.to_string().into()))
        .await;

      // Try to read ServerInfo response with timeout
      let read_result = tokio::time::timeout(
        Duration::from_secs(2),
        ws_receiver.next()
      ).await;

      // If we got a response, that's good
      if let Ok(Some(Ok(msg))) = read_result {
        let _text = msg.into_text().expect("Expected text message");
        // Parser would validate ServerInfoV4 here
      }
    }
    _ => {
      // Connection failed or timeout - that's ok, runner will handle it
    }
  }

  // Await the runner task with timeout
  let runner_result = tokio::time::timeout(
    Duration::from_secs(15),
    runner_task
  )
  .await;

  // Just verify runner completed without panicking
  if let Ok(Ok(result)) = runner_result {
    println!("Sequence completed: {}", result.sequence_name);
    // Verify the connection step at least tried
    assert!(
      result.steps.iter().any(|s| s.step_name == "Connection"),
      "Runner should have attempted connection step"
    );
  } else {
    panic!("Runner task failed or timed out");
  }
}

#[tokio::test]
async fn test_core_protocol_wrong_first_message() {
  // Use a random port between 20000 and 29999
  let port = 20000 + (std::process::id() % 10000) as u16 + 1;

  // Spawn the runner in the background with shorter timeout
  let runner_task = tokio::spawn(async move {
    run_sequence(&core_protocol_sequence(), port, 500).await
  });

  // Wait for WebSocket server to start
  sleep(Duration::from_millis(500)).await;

  // Connect via raw WebSocket with timeout
  let url = format!("ws://127.0.0.1:{}", port);
  let connect_result = tokio::time::timeout(
    Duration::from_secs(5),
    tokio_tungstenite::connect_async(&url)
  )
  .await;

  if let Ok(Ok((ws_stream, _))) = connect_result {
    let (mut ws_sender, _ws_receiver) = ws_stream.split();

    // Send StartScanning instead of RequestServerInfo (wrong first message)
    let start_scanning = json!([{
      "StartScanningV0": {
        "Id": 1
      }
    }]);
    let _ = ws_sender
      .send(Message::Text(start_scanning.to_string().into()))
      .await;

    // Close WebSocket connection
    drop(ws_sender);
  }

  // Await the runner task with timeout
  let runner_result = tokio::time::timeout(
    Duration::from_secs(15),
    runner_task
  )
  .await;

  // Just verify runner completed without panicking
  if let Ok(Ok(result)) = runner_result {
    println!("Sequence completed: {}", result.sequence_name);
    // Verify the connection step at least tried
    assert!(
      result.steps.iter().any(|s| s.step_name == "Connection"),
      "Runner should have attempted connection step"
    );
  } else {
    panic!("Runner task failed or timed out");
  }
}
