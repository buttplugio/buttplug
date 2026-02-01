// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod in_process_connector;
pub use in_process_connector::ButtplugInProcessClientConnectorBuilder;

use buttplug_client::serializer::ButtplugClientJSONSerializer;
use buttplug_core::connector::ButtplugRemoteConnector;
use buttplug_server::message::{ButtplugClientMessageV3, ButtplugServerMessageV3};

pub type ButtplugRemoteClientConnector<
  TransportType,
  SerializerType = ButtplugClientJSONSerializer,
> = ButtplugRemoteConnector<
  TransportType,
  SerializerType,
  ButtplugClientMessageV3,
  ButtplugServerMessageV3,
>;
