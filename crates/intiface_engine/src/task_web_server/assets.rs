// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Embedded, dependency-free static assets. Embedded with `include_str!` so an
//! installed binary needs no adjacent files and makes no external network
//! requests.

pub(crate) const INDEX_HTML: &str = include_str!("assets/index.html");
pub(crate) const APP_CSS: &str = include_str!("assets/app.css");
pub(crate) const APP_JS: &str = include_str!("assets/app.js");
