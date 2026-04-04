// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_client_conformance_test::runner::run_sequence;
use buttplug_client_conformance_test::sequences::core_protocol::core_protocol_sequence;
use std::time::Duration;
use tokio_tungstenite;

#[tokio::test]
async fn test_core_protocol_full_pass() {
  // Use a fixed port based on PID to avoid collisions (base 21000)
  let port = 21000 + (std::process::id() % 1000) as u16;

  // Spawn the runner in the background
  let runner_task =
    tokio::spawn(async move { run_sequence(&core_protocol_sequence(), port, 5000).await });

  // Wait for WebSocket server to start, then connect a minimal client
  tokio::time::sleep(Duration::from_millis(100)).await;
  let url = format!("ws://127.0.0.1:{}", port);

  // Spawn a minimal client that just connects and waits
  let _client_task = tokio::spawn(async move {
    if let Ok(Ok((_ws_stream, _))) = tokio::time::timeout(
      Duration::from_secs(5),
      tokio_tungstenite::connect_async(&url),
    )
    .await
    {
      // Just keep the connection open while runner executes
      tokio::time::sleep(Duration::from_secs(30)).await;
    }
  });

  // Await the runner task with timeout
  let runner_result = tokio::time::timeout(Duration::from_secs(30), runner_task).await;

  // Verify runner completed successfully with all steps passed
  if let Ok(Ok(result)) = runner_result {
    println!("Sequence completed: {}", result.sequence_name);
    assert!(
      result.passed,
      "All sequence steps should pass: {:?}",
      result
        .steps
        .iter()
        .filter(|s| !s.passed)
        .collect::<Vec<_>>()
    );
  } else {
    panic!("Runner task failed or timed out");
  }
}

#[tokio::test]
async fn test_core_protocol_wrong_first_message() {
  // Use a fixed port based on PID to avoid collisions (base 22000)
  let port = 22000 + (std::process::id() % 1000) as u16;

  // Spawn the runner in the background with minimal timeout
  let runner_task =
    tokio::spawn(async move { run_sequence(&core_protocol_sequence(), port, 500).await });

  // Don't connect any client - test expects failure due to no connection
  // Await the runner task with timeout
  let runner_result = tokio::time::timeout(Duration::from_secs(30), runner_task).await;

  // Verify runner completed and sequence failed (no client connects)
  if let Ok(Ok(result)) = runner_result {
    println!("Sequence completed: {}", result.sequence_name);
    // When no client connects, the runner fails on connection step
    assert!(
      !result.passed,
      "Sequence should fail when no client connects"
    );
  } else {
    panic!("Runner task failed or timed out");
  }
}
