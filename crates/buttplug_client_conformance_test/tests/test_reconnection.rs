// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_client_conformance_test::runner::run_sequence;
use buttplug_client_conformance_test::sequences::reconnection::reconnection_sequence;
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use std::time::Duration;
use tokio_tungstenite;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn test_reconnection_pass() {
  // Use port 25000 + (PID % 1000) to avoid collisions
  let port = 25000 + (std::process::id() % 1000) as u16;

  // Spawn the runner in the background with generous timeout
  let runner_task = tokio::spawn(async move {
    run_sequence(&reconnection_sequence(), port, 45000).await
  });

  // Wait for WebSocket server to start
  tokio::time::sleep(Duration::from_millis(100)).await;

  let url = format!("ws://127.0.0.1:{}", port);
  let url_clone = url.clone();

  // Spawn first client connection task
  let first_client = tokio::spawn(async move {
    if let Ok(Ok((mut ws_stream, _))) = tokio::time::timeout(
      Duration::from_secs(5),
      tokio_tungstenite::connect_async(&url),
    )
    .await
    {
      // Send RequestServerInfo to trigger handshake
      let handshake_msg = r#"[{"RequestServerInfo":{"Id":1,"ClientName":"Test1","ProtocolVersionMajor":4}}]"#;
      if let Ok(_) = ws_stream.send(Message::Text(handshake_msg.into())).await {
        // Receive ServerInfo response
        if let Some(Ok(_msg)) = ws_stream.next().await {
          // Start scanning
          let start_scan_msg = r#"[{"StartScanning":{"Id":2}}]"#;
          let _ = ws_stream.send(Message::Text(start_scan_msg.into())).await;

          // Wait for scan to complete
          tokio::time::sleep(Duration::from_secs(1)).await;

          // Consume scan response messages
          while let Ok(Some(_)) = tokio::time::timeout(
            Duration::from_millis(100),
            ws_stream.next(),
          )
          .await
          {
            // Drain messages
          }

          // Now wait for server to close connection
          // The runner will issue CloseConnection which closes the server connection
          while ws_stream.next().await.is_some() {
            tokio::time::sleep(Duration::from_millis(100)).await;
          }
        }
      }
    }
  });

  // Wait for server rebuild to occur
  tokio::time::sleep(Duration::from_secs(5)).await;

  // Spawn second client connection task (after server rebuild)
  let second_client = tokio::spawn(async move {
    if let Ok(Ok((mut ws_stream, _))) = tokio::time::timeout(
      Duration::from_secs(5),
      tokio_tungstenite::connect_async(&url_clone),
    )
    .await
    {
      // Send RequestServerInfo to trigger handshake on new connection
      let handshake_msg = r#"[{"RequestServerInfo":{"Id":1,"ClientName":"Test2","ProtocolVersionMajor":4}}]"#;
      if let Ok(_) = ws_stream.send(Message::Text(handshake_msg.into())).await {
        // Receive ServerInfo response
        if let Some(Ok(_msg)) = ws_stream.next().await {
          // Start scanning
          let start_scan_msg = r#"[{"StartScanning":{"Id":2}}]"#;
          let _ = ws_stream.send(Message::Text(start_scan_msg.into())).await;

          // Wait for scan to complete
          tokio::time::sleep(Duration::from_secs(1)).await;

          // Consume scan response messages
          while let Ok(Some(_)) = tokio::time::timeout(
            Duration::from_millis(100),
            ws_stream.next(),
          )
          .await
          {
            // Drain messages
          }

          // Send device command to device 0
          let device_cmd_msg = r#"[{"OutputCmd":{"Id":3,"DeviceIndex":0,"Endpoints":[{"Index":0,"Data":[128]}]}}]"#;
          let _ = ws_stream.send(Message::Text(device_cmd_msg.into())).await;

          // Receive ok response
          let _ = ws_stream.next().await;

          // Keep connection open
          tokio::time::sleep(Duration::from_secs(10)).await;
        }
      }
    }
  });

  // Wait for runner with generous timeout (45 seconds for two full connections)
  let runner_result = tokio::time::timeout(Duration::from_secs(50), runner_task).await;

  // Verify runner completed successfully
  if let Ok(Ok(result)) = runner_result {
    println!("Reconnection sequence completed: {}", result.sequence_name);
    assert!(
      result.passed,
      "Reconnection sequence should pass. Failed steps: {:?}",
      result
        .steps
        .iter()
        .filter(|s| !s.passed)
        .map(|s| &s.step_name)
        .collect::<Vec<_>>()
    );
  } else {
    panic!("Reconnection runner task failed or timed out");
  }

  // Clean up client tasks
  let _ = tokio::time::timeout(Duration::from_secs(1), first_client).await;
  let _ = tokio::time::timeout(Duration::from_secs(1), second_client).await;
}
