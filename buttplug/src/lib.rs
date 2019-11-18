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
//! Welcome to the Buttplug Intimate Sex Toy Control Library. I'm your host,
//! qDot.
//!
//! ## But why?
//!
//! So maybe you're thinking "But why do we need a sex toy control library?"
//! Well, there's a bunch of sex toys out there that talk to computers, and they
//! all talk in different ways, and it's annoying. So this library tries to fix
//! that, so when you write an app that controls a sex toy, it can control as
//! many toys as possible.
//!
//! If you've worked with programs like [OSCulator](https://osculator.net/) or
//! [FreePIE](https://andersmalmgren.github.io/FreePIE/), consider Buttplug
//! similar to those, just for another niche of hardware.
//!
//! ## But why Rust?
//!
//! At the time of this writing, we already have perfectly functional Buttplug
//! implementations in C# and Typescript. So why Rust?
//!
//! Both C# and TS/JS are languages that require runtimes, while Rust is a
//! compiled systems language. This makes cross-platform distribution difficult.
//! It also means we may be tied to certain runtime versions for other needs
//! (i.e. our C# implementation doesn't support older Unity games). Going all
//! RIIR on Buttplug means that we can keep one implementation of our core logic
//! in Rust, then hopefully FFI (or WASM) to other languages.
//!
//! Note that this does mean Buttplug was designed with garbage collected
//! languages in mind first, something Rust doesn't have built-in. Some of our
//! idioms that require things like event handlers will look slightly different
//! in Rust, but that's ok! They'll still work. But if you're looking at
//! examples in other languages, you may have to do some translation.
//!
//! Rust also gives us more guarantees about concurrency and safety than our
//! other implementations, which is good because this software goes in butts.
//!
//! ## Recommended Reading
//!
//! Before diving into the library, there's a couple of things you might want to
//! check out.
//!
//! - [Buttplug Protocol Spec](https://buttplug-spec.docs.buttplug.io) - This is
//! the low level network protocol spec for Buttplug. While using this library,
//! you'll rarely run into having to form this level of messages yourself (it's
//! why we have an API, after all), but this spec informs the architecture of
//! the system as a whole, so it's good to be familiar with.
//!
//! - [Buttplug Developer
//! Guide](https://buttplug-developer-guide.docs.buttplug.io) - A guide for
//! developers who are interested in using Buttplug. Goes over basic application
//! structure and library usage.
//!
//! ## So what can I do with this?
//!
//! Currently the only thing that's implemented in this library is about half of
//! the client API, and it uses a bunch of preview/beta stuff like async/await,
//! async-std, etc... So the answer is, "Not much".
//!
//! Most of the development happening right now is experimental to see how Rust
//! will work for our needs, both as a library and as an FFI'able implementation
//! to set other languages on top of. As development continues, hopefully I'll
//! remember to update this section to say that things are actually usable at
//! some point.
//!
//! But I probably won't.
//!
//! ## How can I help?
//!
//! Right now, we mostly need code/API style reviews and feedback. We don't
//! really have any good bite-sized chunks to apportion out on the
//! implementation yet, but one we do, those will be marked "Help Wanted" in our
//! [github issues](https://github.com/buttplugio/buttplug-rs/issues).

#[macro_use]
extern crate buttplug_derive;
#[macro_use]
extern crate log;

pub mod core;
#[cfg(feature="client")]
pub mod client;
#[cfg(feature="server")]
pub mod server;
