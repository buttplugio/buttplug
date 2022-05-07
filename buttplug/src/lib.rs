// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#![crate_type = "lib"]
#![crate_name = "buttplug"]
// Required for select! expansion in RemoteServer
#![recursion_limit = "512"]
#![doc = include_str!("../README.md")]

#[macro_use]
extern crate buttplug_derive;
#[macro_use]
extern crate strum_macros;
#[cfg(any(feature = "client", feature = "server"))]
#[macro_use]
extern crate futures;
#[macro_use]
extern crate tracing;

#[cfg(feature = "client")]
pub mod client;
#[cfg(any(feature = "client", feature = "server"))]
pub mod connector;
pub mod core;
pub mod device;
#[cfg(feature = "server")]
pub mod server;
pub mod util;
