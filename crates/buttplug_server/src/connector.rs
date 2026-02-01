// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::connector::ButtplugRemoteConnector;

use super::message::{ButtplugClientMessageVariant, ButtplugServerMessageVariant};

pub type ButtplugRemoteServerConnector<TransportType, SerializerType> = ButtplugRemoteConnector<
  TransportType,
  SerializerType,
  ButtplugServerMessageVariant,
  ButtplugClientMessageVariant,
>;
