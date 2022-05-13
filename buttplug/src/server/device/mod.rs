// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Device identification and configuration, and protocol definitions
//!
//! Structs in the device module are used by the [Buttplug Server](crate::server) (specifically
//! the [Device Manager](crate::server::device_manager::DeviceManager)) to identify devices that
//! Buttplug can connect to, and match them to supported protocols in order to establish
//! communication.

pub mod communication;
pub mod manager;
pub mod configuration;
pub mod device;
pub mod protocol;