// Is this just two examples from tokio_tungstenite glued together?
//
// It absolute is!

use futures_util::{future, StreamExt, TryStreamExt};
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

  pub async fn listen(&self) {
    info!("Repeater loop starting");
    let addr = format!("127.0.0.1:{}", self.local_port);

    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
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
  }

  async fn accept_connection(server_addr: String, stream: TcpStream) {
    let client_addr = stream
      .peer_addr()
      .expect("connected streams should have a peer address");
    info!("Client address: {}", client_addr);

    let client_ws_stream = tokio_tungstenite::accept_async(stream)
      .await
      .expect("Error during the websocket handshake occurred");

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
