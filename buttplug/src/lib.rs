// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#![crate_type = "lib"]
#![crate_name = "buttplug"]

//! # Buttplug Intimate Sex Toy Control Library
//!
//! [![Patreon donate button](https://img.shields.io/badge/patreon-donate-yellow.svg)](https://www.patreon.com/qdot)
//! [![Github donate button](https://img.shields.io/badge/github-donate-ff69b4.svg)](https://www.github.com/sponsors/qdot)
//! [![Discourse Forum](https://img.shields.io/badge/discourse-forum-blue.svg)](https://metafetish.club)
//! [![Discord](https://img.shields.io/discord/353303527587708932.svg?logo=discord)](https://discord.buttplug.io)
//! [![Twitter](https://img.shields.io/twitter/follow/buttplugio.svg?style=social&logo=twitter)](https://twitter.com/buttplugio)
//!
//! [![Crates.io Version](https://img.shields.io/crates/v/buttplug)](https://crates.io/crates/buttplug)
//! [![Crates.io Downloads](https://img.shields.io/crates/d/buttplug)](https://crates.io/crates/buttplug)
//! [![Crates.io License](https://img.shields.io/crates/l/buttplug)](https://crates.io/crates/buttplug)
//!
//! Welcome to the Buttplug Intimate Sex Toy Control Library.
//!
//! If you're here, we're assuming you know why you're here and will dispense
//! with the "this is what this library is" stuff.
//!
//! If you don't know why you're here, check out [our main
//! website](https://buttplug.io) or [our github
//! repo](https://github.com/buttplugio/buttplug-rs) for more introductory
//! information.
//!
//! ## Requirements
//!
//! buttplug-rs uses async/await heavily, and requires a minimum of Rust 1.39.
//!
//! While we use [async-std](https://async.rs/) internally, buttplug-rs should
//! work with any runtime.
//!
//! ## Currently Implemented Capabilities
//!
//! The library currently contains a complete implementation of the Buttplug
//! Client, which allows connecting to Buttplug Servers (currently written in
//! [C#](https://github.com/buttplugio/buttplug-csharp) and
//! [JS](https://github.com/buttplugio/buttplug-js)), then enumerating and
//! controlling devices after successful connection. There are also connectors
//! included for connecting to servers via Websockets.
//!
//! ## Examples
//!
//! Code examples are available in the [github
//! repo](https://github.com/buttplugio/buttplug-rs/tree/master/buttplug/examples).
//!
//! The [Buttplug Developer
//! Guide](https://buttplug-developer-guide.docs.buttplug.io) may also be
//! useful, though it does not currently have Rust examples.
//!
//! ## Attributes
//!
//! The following attributes are available
//!
//! | Feature | Other Features Used | Description |
//! | --------- | ----------- | ----------- |
//! |  `client` | None | Buttplug client implementation (in-process connection only) |
//! | `server` | None | Buttplug server implementation (in-process connection only) |
//! | `serialize_json` | None | Serde JSON serializer for Buttplug messages, needed for remote connectors |
//! | `client-ws` | `client`,`serialize_json` | Websocket client connector, used to connect clients to remote servers |
//! | `client-ws-ssl` | `client`,`serialize_json` | Websocket client connector with SSL capabilities |
//!
//! Default attributes are `client-ws` and `server`.
//!
//! ## Plans for the Future
//!
//! The next 2 goals are:
//!
//! - Creating an FFI layer so that we can build other language libraries on top
//! of this implementation.
//! - Writing the server portion in Rust.
//!
//! These will be happening simultaneously after the v0.0.2 release.
//!
//! ## Contributing
//!
//! Right now, we mostly need code/API style reviews and feedback. We don't
//! really have any good bite-sized chunks to apportion out on the
//! implementation yet, but one we do, those will be marked "Help Wanted" in our
//! [github issues](https://github.com/buttplugio/buttplug-rs/issues).

#[macro_use]
extern crate buttplug_derive;
#[macro_use]
extern crate strum_macros;
#[cfg(feature = "thread_pool_runtime")]
#[macro_use]
extern crate lazy_static;
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

pub mod test;
