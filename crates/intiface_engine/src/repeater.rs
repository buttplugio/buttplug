// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

// Is this just two examples from tokio_tungstenite glued together?
//
// It absolute is!

use futures_util::{StreamExt, TryStreamExt, future};
use log::info;
use tokio::{
  net::{TcpListener, TcpStream},
  select,
};
use tokio_tungstenite::connect_async;
use tokio_util::sync::CancellationToken;

pub struct ButtplugRepeater {
  local_port: u16,
  remote_address: String,
  stop_token: CancellationToken,
}

impl ButtplugRepeater {
  pub fn new(local_port: u16, remote_address: &str, stop_token: CancellationToken) -> Self {
    Self {
      local_port,
      remote_address: remote_address.to_owned(),
      stop_token,
    }
  }

  pub async fn listen(&self) -> Result<(), std::io::Error> {
    info!("Repeater loop starting");
    let addr = format!("127.0.0.1:{}", self.local_port);

    let listener = TcpListener::bind(&addr).await?;
    info!("Listening on: {}", addr);

    loop {
      select! {
        stream_result = listener.accept() => {
          match stream_result {
            Ok((stream, _)) => {
              let mut remote_address = self.remote_address.clone();
              if !remote_address.starts_with("ws://") {
                remote_address.insert_str(0, "ws://");
              }
              tokio::spawn(ButtplugRepeater::accept_connection(remote_address, stream));
            },
            Err(e) => {
              error!("Error accepting new websocket for repeater: {:?}", e);
              break;
            }
          }
        },
        _ = self.stop_token.cancelled() => {
          info!("Repeater loop requested to stop, breaking.");
          break;
        }
      }
    }
    info!("Repeater loop exiting");
    Ok(())
  }

  async fn accept_connection(server_addr: String, stream: TcpStream) {
    let client_addr = match stream.peer_addr() {
      Ok(addr) => addr,
      Err(err) => {
        error!("Cannot get repeater client address: {:?}", err);
        return;
      }
    };
    info!("Client address: {}", client_addr);

    let client_ws_stream = match tokio_tungstenite::accept_async(stream).await {
      Ok(stream) => stream,
      Err(err) => {
        error!(
          "Error during repeater websocket handshake with {}: {:?}",
          client_addr, err
        );
        return;
      }
    };

    info!("New WebSocket connection: {}", client_addr);

    info!("Connecting to server {}", server_addr);

    let server_url = url::Url::parse(&server_addr).unwrap();

    let ws_stream = match connect_async(&server_url).await {
      Ok((stream, _)) => stream,
      Err(e) => {
        error!("Cannot connect: {:?}", e);
        return;
      }
    };
    info!("WebSocket handshake has been successfully completed");

    let (server_write, server_read) = ws_stream.split();

    let (client_write, client_read) = client_ws_stream.split();

    let client_fut = client_read
      .try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()))
      .forward(server_write);
    let server_fut = server_read
      .try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()))
      .forward(client_write);
    future::select(client_fut, server_fut).await;
    info!("Closing repeater connection.");
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio::{io::AsyncWriteExt, net::TcpListener};

  #[tokio::test]
  async fn accept_connection_returns_on_incomplete_websocket_handshake() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let accept_task = tokio::spawn(async move {
      let (stream, _) = listener.accept().await.unwrap();
      ButtplugRepeater::accept_connection("ws://127.0.0.1:1".to_owned(), stream).await;
    });

    let mut client = TcpStream::connect(addr).await.unwrap();
    client
      .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\n")
      .await
      .unwrap();
    client.shutdown().await.unwrap();

    assert!(accept_task.await.is_ok());
  }
}
