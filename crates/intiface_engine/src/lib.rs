// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#[macro_use]
extern crate log;

mod backdoor_server;
mod buttplug_server;
mod engine;
mod error;
mod frontend;
mod mdns;
mod options;
mod remote_server;
mod repeater;
mod rest_server;
pub use backdoor_server::BackdoorServer;
pub use engine::IntifaceEngine;
pub use error::*;
pub use frontend::{EngineMessage, Frontend, IntifaceMessage};
pub use options::{EngineOptions, EngineOptionsBuilder, EngineOptionsExternal};
pub use remote_server::{ButtplugRemoteServer, ButtplugServerConnectorError};
pub use repeater::ButtplugRepeater;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use buttplug_server_hwmgr_serial::{AvailableSerialPort, available_serial_ports};

#[cfg(any(target_os = "android", target_os = "ios"))]
mod mobile_serial_port {
  #[derive(Debug, Clone)]
  pub struct AvailableSerialPort {
    pub port_name: String,
    pub port_type: String,
    pub vid: Option<u16>,
    pub pid: Option<u16>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
  }

  pub fn available_serial_ports() -> Vec<AvailableSerialPort> {
    vec![]
  }
}
#[cfg(any(target_os = "android", target_os = "ios"))]
pub use mobile_serial_port::{AvailableSerialPort, available_serial_ports};
