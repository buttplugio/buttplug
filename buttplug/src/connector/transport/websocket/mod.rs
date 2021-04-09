pub mod websocket_client;
#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
pub mod websocket_server;

pub use async_tungstenite::tungstenite::Error as TungsteniteError;
pub use websocket_client::ButtplugWebsocketClientTransport;
#[cfg(any(feature = "async-std-runtime", feature = "tokio-runtime"))]
pub use websocket_server::{
  ButtplugWebsocketServerTransport,
  ButtplugWebsocketServerTransportOptions,
};
