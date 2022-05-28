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

//! # An Overview of Buttplug's Module System
//!
//! Buttplug is broken up into the following modules:
//!
//! - [Core](crate::core)
//!   - Generic portions of the library code that are used by the other modules. This includes
//!     message classes, serializers, connectors, and errors.
//! - [Client](crate::client)
//!   - The public facing API for applications. This module is what most programs will use to talk
//!     to Buttplug servers, either directly through Rust, or through our [FFI
//!     Layer](https://github.com/buttplugio/buttplug-rs-ffi) for other languages.
//! - [Server](crate::server)
//!   - Handles actual hardware connections and communication. If you want to add new devices or
//!     protocols to Buttplug, or change how the system access devices, this is the module you'll be
//!     working in.
//! - [Util](crate::util)
//!   - Utilities for all portions of the library that may not be specifically related to sex toy
//!     functionality. This includes managers for different async runtimes, configuration file
//!     loading, utilities for streams and futures, etc...

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
pub mod core;
#[cfg(feature = "server")]
pub mod server;
pub mod util;
