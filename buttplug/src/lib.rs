// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#![crate_type = "lib"]
#![crate_name = "buttplug"]

#[macro_use]
extern crate buttplug_derive;
#[macro_use]
extern crate log;

pub mod client;
pub mod core;
pub mod server;
