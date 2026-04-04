// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_client_conformance_test::runner::run_sequence;
use buttplug_client_conformance_test::sequences::ping_required::ping_required_sequence;
use buttplug_client_conformance_test::sequences::ping_timeout::ping_timeout_sequence;
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use std::time::Duration;
use tokio_tungstenite;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn test_ping_required_pass() {
  // Use port 23000 + (PID % 1000) to avoid collisions
  let port = 23000 + (std::process::id() % 1000) as u16;

  // Spawn the runner in the background
  let runner_task = tokio::spawn(async move {
    run_sequence(&ping_required_sequence(), port, 10000).await
  });

  // Wait for WebSocket server to start
  tokio::time::sleep(Duration::from_millis(100)).await;

  let url = format!("ws://127.0.0.1:{}", port);

  // Spawn a client that connects and sends periodic pings
  let client_task = tokio::spawn(async move {
    if let Ok(Ok((mut ws_stream, _))) = tokio::time::timeout(
      Duration::from_secs(5),
      tokio_tungstenite::connect_async(&url),
    )
    .await
    {
      // Send RequestServerInfo to trigger handshake
      let handshake_msg = r#"[{"RequestServerInfo":{"Id":1,"ClientName":"Test","ProtocolVersionMajor":4}}]"#;
      if let Ok(_) = ws_stream.send(Message::Text(handshake_msg.into())).await {
        // Receive ServerInfo response
        if let Some(Ok(_msg)) = ws_stream.next().await {
          // Now start sending pings every 500ms (well within 1000ms max_ping_time)
          for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Send PingV0 message
            let ping_msg = r#"[{"Ping":{"Id":2}}]"#;
            if let Err(_) = ws_stream.send(Message::Text(ping_msg.into())).await {
              // Connection closed
              break;
            }

            // Try to receive response (Ok message)
            if ws_stream.next().await.is_none() {
              break;
            }
          }
        }
      }

      // Keep connection open for test duration
      tokio::time::sleep(Duration::from_secs(10)).await;
    }
  });

  // Wait for runner with timeout
  let runner_result = tokio::time::timeout(Duration::from_secs(15), runner_task).await;

  // Verify runner completed successfully
  if let Ok(Ok(result)) = runner_result {
    println!("Ping required sequence completed: {}", result.sequence_name);
    assert!(
      result.passed,
      "Ping required sequence should pass. Failed steps: {:?}",
      result
        .steps
        .iter()
        .filter(|s| !s.passed)
        .map(|s| &s.step_name)
        .collect::<Vec<_>>()
    );
  } else {
    panic!("Ping required runner task failed or timed out");
  }

  // Clean up client task
  let _ = tokio::time::timeout(Duration::from_secs(1), client_task).await;
}

#[tokio::test]
async fn test_ping_timeout_pass() {
  // Use port 23001 + (PID % 1000) to avoid collisions
  let port = 23001 + (std::process::id() % 1000) as u16;

  // Spawn the runner in the background
  let runner_task = tokio::spawn(async move {
    run_sequence(&ping_timeout_sequence(), port, 15000).await
  });

  // Wait for WebSocket server to start
  tokio::time::sleep(Duration::from_millis(100)).await;

  let url = format!("ws://127.0.0.1:{}", port);

  // Spawn a client that connects but DOES NOT send pings
  let client_task = tokio::spawn(async move {
    if let Ok(Ok((mut ws_stream, _))) = tokio::time::timeout(
      Duration::from_secs(5),
      tokio_tungstenite::connect_async(&url),
    )
    .await
    {
      // Send RequestServerInfo to trigger handshake
      let handshake_msg = r#"[{"RequestServerInfo":{"Id":1,"ClientName":"Test","ProtocolVersionMajor":4}}]"#;
      if let Ok(_) = ws_stream.send(Message::Text(handshake_msg.into())).await {
        // Receive ServerInfo response
        if let Some(Ok(_msg)) = ws_stream.next().await {
          // Now just wait for server to disconnect due to ping timeout
          // We DO NOT send any Ping messages
          tokio::time::sleep(Duration::from_secs(3)).await;

          // Try to read until connection closes
          while ws_stream.next().await.is_some() {
            tokio::time::sleep(Duration::from_millis(100)).await;
          }
        }
      }
    }
  });

  // Wait for runner with generous timeout (15 seconds for ping timeout)
  let runner_result = tokio::time::timeout(Duration::from_secs(20), runner_task).await;

  // Verify runner completed and sequence passed
  if let Ok(Ok(result)) = runner_result {
    println!("Ping timeout sequence completed: {}", result.sequence_name);
    assert!(
      result.passed,
      "Ping timeout sequence should pass (server correctly detected timeout). Failed steps: {:?}",
      result
        .steps
        .iter()
        .filter(|s| !s.passed)
        .map(|s| &s.step_name)
        .collect::<Vec<_>>()
    );
  } else {
    panic!("Ping timeout runner task failed or timed out");
  }

  // Clean up client task
  let _ = tokio::time::timeout(Duration::from_secs(1), client_task).await;
}
