// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::serializer::ButtplugClientJSONSerializer;
use buttplug_core::{
  connector::ButtplugRemoteConnector,
  message::{ButtplugClientMessageV4, ButtplugServerMessageV4},
};

pub type ButtplugRemoteClientConnector<
  TransportType,
  SerializerType = ButtplugClientJSONSerializer,
> = ButtplugRemoteConnector<
  TransportType,
  SerializerType,
  ButtplugClientMessageV4,
  ButtplugServerMessageV4,
>;
