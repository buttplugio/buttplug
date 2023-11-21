// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Websocket connector for client/server communication

pub mod websocket_client;
pub mod websocket_server;

pub use tokio_tungstenite::tungstenite::Error as TungsteniteError;
pub use websocket_client::ButtplugWebsocketClientTransport;

pub use websocket_server::{
  ButtplugWebsocketServerTransport,
  ButtplugWebsocketServerTransportBuilder,
};
