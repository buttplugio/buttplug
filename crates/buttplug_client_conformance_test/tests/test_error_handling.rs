// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_client_conformance_test::runner::run_sequence;
use buttplug_client_conformance_test::sequences::error_handling::error_handling_sequence;
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use std::time::Duration;
use tokio_tungstenite;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn test_error_handling_pass() {
  // Use port 24000 + (PID % 1000) to avoid collisions
  let port = 24000 + (std::process::id() % 1000) as u16;

  // Spawn the runner in the background
  let runner_task =
    tokio::spawn(async move { run_sequence(&error_handling_sequence(), port, 10000).await });

  // Wait for WebSocket server to start
  tokio::time::sleep(Duration::from_millis(100)).await;

  let url = format!("ws://127.0.0.1:{}", port);

  // Spawn a client that connects and holds the connection open
  let client_task = tokio::spawn(async move {
    if let Ok(Ok((mut ws_stream, _))) = tokio::time::timeout(
      Duration::from_secs(5),
      tokio_tungstenite::connect_async(&url),
    )
    .await
    {
      // Send RequestServerInfo to trigger handshake
      let handshake_msg =
        r#"[{"RequestServerInfo":{"Id":1,"ClientName":"Test","ProtocolVersionMajor":4}}]"#;
      if let Ok(_) = ws_stream.send(Message::Text(handshake_msg.into())).await {
        // Receive ServerInfo response
        if let Some(Ok(_msg)) = ws_stream.next().await {
          // Keep connection open and drain any incoming messages for the entire test
          loop {
            match tokio::time::timeout(Duration::from_millis(100), ws_stream.next()).await {
              Ok(Some(_)) => {
                // Got a message, continue
              }
              Ok(None) => {
                // Connection closed by server
                break;
              }
              Err(_) => {
                // Timeout, continue waiting
              }
            }
          }
        }
      }
    }
  });

  // Wait for runner with timeout (give plenty of time for device commands)
  let runner_result = tokio::time::timeout(Duration::from_secs(20), runner_task).await;

  // Verify runner completed successfully
  if let Ok(Ok(result)) = runner_result {
    println!(
      "Error handling sequence completed: {}",
      result.sequence_name
    );
    let failed_steps: Vec<_> = result.steps.iter().filter(|s| !s.passed).collect();

    if !failed_steps.is_empty() {
      println!("Failed steps details:");
      for step in &failed_steps {
        println!("  - {}: {:?}", step.step_name, step.error);
      }
    }

    // Also print passed steps for context
    println!("All steps:");
    for step in &result.steps {
      println!(
        "  - {} ({}ms): {}",
        step.step_name,
        step.duration_ms,
        if step.passed { "PASS" } else { "FAIL" }
      );
    }

    assert!(
      result.passed,
      "Error handling sequence should pass. Failed steps: {:?}",
      failed_steps
        .iter()
        .map(|s| format!(
          "{} ({})",
          s.step_name,
          s.error.as_deref().unwrap_or("unknown")
        ))
        .collect::<Vec<_>>()
    );
  } else {
    panic!("Error handling runner task failed or timed out");
  }

  // Clean up client task
  let _ = tokio::time::timeout(Duration::from_secs(1), client_task).await;
}
