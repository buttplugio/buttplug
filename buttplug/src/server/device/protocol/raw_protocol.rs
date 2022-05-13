// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler, GenericCommandManager};
use crate::{
  core::messages::{ButtplugDeviceCommandMessageUnion, },
  server::device::{
    device::device_impl::DeviceImpl,
    protocol::ButtplugProtocolProperties,
    configuration::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder},
  },
};
use std::sync::Arc;

super::default_protocol_declaration!(RawProtocol, "raw");

impl ButtplugProtocolCommandHandler for RawProtocol {
}

// TODO Write tests
