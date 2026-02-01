// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#[macro_use]
extern crate log;

mod in_process_client;
mod in_process_connector;

pub use in_process_client::in_process_client;
pub use in_process_connector::{
  ButtplugInProcessClientConnector,
  ButtplugInProcessClientConnectorBuilder,
};
