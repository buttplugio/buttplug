// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::protocol::{generic_protocol_setup, ProtocolHandler};

generic_protocol_setup!(RawProtocol, "raw");

#[derive(Default)]
pub struct RawProtocol {}

impl ProtocolHandler for RawProtocol {
}

// TODO Write tests
